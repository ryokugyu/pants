use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use bazel_protos;
use futures;
use grpcio;

use bytes::Bytes;
use futures::{Future, IntoFuture, Stream};
use hashing::{Digest, Fingerprint};
use testutil::data::{TestData, TestDirectory};

///
/// Implements the ContentAddressableStorage gRPC API, answering read requests with either known
/// content, NotFound for valid but unknown content, or InvalidArguments for bad arguments.
///
pub struct StubCAS {
  server_transport: grpcio::Server,
  read_request_count: Arc<Mutex<usize>>,
  pub write_message_sizes: Arc<Mutex<Vec<usize>>>,
  pub blobs: Arc<Mutex<HashMap<Fingerprint, Bytes>>>,
}

impl StubCAS {
  pub fn with_content(
    chunk_size_bytes: i64,
    files: Vec<TestData>,
    directories: Vec<TestDirectory>,
  ) -> StubCAS {
    let mut blobs = HashMap::new();
    for file in files {
      blobs.insert(file.fingerprint(), file.bytes());
    }
    for directory in directories {
      blobs.insert(directory.fingerprint(), directory.bytes());
    }
    StubCAS::with_unverified_content(chunk_size_bytes, blobs)
  }

  ///
  /// Wrapper around with_unverified_content_and_port
  ///
  pub fn with_unverified_content(
    chunk_size_bytes: i64,
    blobs: HashMap<Fingerprint, Bytes>,
  ) -> StubCAS {
    StubCAS::with_unverified_content_and_port(chunk_size_bytes, blobs, 0)
  }

  ///
  /// # Arguments
  /// * `chunk_size_bytes` - The maximum number of bytes of content to include per streamed message.
  ///                        Messages will saturate until the last one, which may be smaller than
  ///                        this value.
  ///                        If a negative value is given, all requests will receive an error.
  /// * `blobs`            - Known Fingerprints and their content responses. These are not checked
  ///                        for correctness.
  /// * `port`             - The port for the CAS to listen to.
  pub fn with_unverified_content_and_port(
    chunk_size_bytes: i64,
    blobs: HashMap<Fingerprint, Bytes>,
    port: u16,
  ) -> StubCAS {
    let env = Arc::new(grpcio::Environment::new(1));
    let read_request_count = Arc::new(Mutex::new(0));
    let write_message_sizes = Arc::new(Mutex::new(Vec::new()));
    let blobs = Arc::new(Mutex::new(blobs));
    let responder = StubCASResponder {
      chunk_size_bytes: chunk_size_bytes,
      blobs: blobs.clone(),
      read_request_count: read_request_count.clone(),
      write_message_sizes: write_message_sizes.clone(),
    };
    let mut server_transport = grpcio::ServerBuilder::new(env)
      .register_service(bazel_protos::bytestream_grpc::create_byte_stream(
        responder.clone(),
      ))
      .register_service(
        bazel_protos::remote_execution_grpc::create_content_addressable_storage(responder.clone()),
      )
      .bind("localhost", port)
      .build()
      .unwrap();
    server_transport.start();

    StubCAS {
      server_transport,
      read_request_count,
      write_message_sizes,
      blobs,
    }
  }

  pub fn with_roland_and_directory(chunk_size_bytes: i64) -> StubCAS {
    StubCAS::with_content(
      chunk_size_bytes,
      vec![TestData::roland()],
      vec![TestDirectory::containing_roland()],
    )
  }

  pub fn empty() -> StubCAS {
    StubCAS::with_unverified_content(1024, HashMap::new())
  }

  pub fn empty_with_port(port: u16) -> StubCAS {
    StubCAS::with_unverified_content_and_port(1024, HashMap::new(), port)
  }

  pub fn always_errors() -> StubCAS {
    StubCAS::with_unverified_content(-1, HashMap::new())
  }

  ///
  /// The address on which this server is listening over insecure HTTP transport.
  ///
  pub fn address(&self) -> String {
    let bind_addr = self.server_transport.bind_addrs().first().unwrap();
    format!("{}:{}", bind_addr.0, bind_addr.1)
  }

  pub fn read_request_count(&self) -> usize {
    *self.read_request_count.lock().unwrap()
  }
}

#[derive(Clone, Debug)]
pub struct StubCASResponder {
  chunk_size_bytes: i64,
  blobs: Arc<Mutex<HashMap<Fingerprint, Bytes>>>,
  pub read_request_count: Arc<Mutex<usize>>,
  pub write_message_sizes: Arc<Mutex<Vec<usize>>>,
}

impl StubCASResponder {
  fn should_always_fail(&self) -> bool {
    self.chunk_size_bytes < 0
  }

  fn read_internal(
    &self,
    req: &bazel_protos::bytestream::ReadRequest,
  ) -> Result<Vec<bazel_protos::bytestream::ReadResponse>, grpcio::RpcStatus> {
    let parts: Vec<_> = req.get_resource_name().splitn(4, '/').collect();
    if parts.len() != 4 || parts.get(0) != Some(&"") || parts.get(1) != Some(&"blobs") {
      return Err(grpcio::RpcStatus::new(
        grpcio::RpcStatusCode::InvalidArgument,
        Some(format!(
          "Bad resource name format {} - want /blobs/some-sha256/size",
          req.get_resource_name()
        )),
      ));
    }
    let digest = parts[2];
    let fingerprint = Fingerprint::from_hex_string(digest).map_err(|e| {
      grpcio::RpcStatus::new(
        grpcio::RpcStatusCode::InvalidArgument,
        Some(format!("Bad digest {}: {}", digest, e)),
      )
    })?;
    if self.should_always_fail() {
      return Err(grpcio::RpcStatus::new(
        grpcio::RpcStatusCode::Internal,
        Some("StubCAS is configured to always fail".to_owned()),
      ));
    }
    let blobs = self.blobs.lock().unwrap();
    let maybe_bytes = blobs.get(&fingerprint);
    match maybe_bytes {
      Some(bytes) => Ok(
        bytes
          .chunks(self.chunk_size_bytes as usize)
          .map(|b| {
            let mut resp = bazel_protos::bytestream::ReadResponse::new();
            resp.set_data(Bytes::from(b));
            resp
          })
          .collect(),
      ),
      None => Err(grpcio::RpcStatus::new(
        grpcio::RpcStatusCode::NotFound,
        Some(format!("Did not find digest {}", fingerprint)),
      )),
    }
  }

  ///
  /// Sends a stream of responses down a sink, in ctx's threadpool.
  ///
  fn send<Item, S>(
    &self,
    ctx: &grpcio::RpcContext,
    sink: grpcio::ServerStreamingSink<Item>,
    stream: S,
  ) where
    Item: Send + 'static,
    S: Stream<Item = (Item, grpcio::WriteFlags), Error = grpcio::Error> + Send + 'static,
  {
    ctx.spawn(stream.forward(sink).map(|_| ()).map_err(|_| ()));
  }
}

impl bazel_protos::bytestream_grpc::ByteStream for StubCASResponder {
  fn read(
    &self,
    ctx: grpcio::RpcContext,
    req: bazel_protos::bytestream::ReadRequest,
    sink: grpcio::ServerStreamingSink<bazel_protos::bytestream::ReadResponse>,
  ) {
    {
      let mut request_count = self.read_request_count.lock().unwrap();
      *request_count += 1;
    }
    match self.read_internal(&req) {
      Ok(response) => self.send(
        &ctx,
        sink,
        futures::stream::iter_ok(
          response
            .into_iter()
            .map(|chunk| (chunk, grpcio::WriteFlags::default())),
        ),
      ),
      Err(err) => {
        sink.fail(err);
      }
    }
  }

  fn write(
    &self,
    ctx: grpcio::RpcContext,
    stream: grpcio::RequestStream<bazel_protos::bytestream::WriteRequest>,
    sink: grpcio::ClientStreamingSink<bazel_protos::bytestream::WriteResponse>,
  ) {
    let should_always_fail = self.should_always_fail();
    let write_message_sizes = self.write_message_sizes.clone();
    let blobs = self.blobs.clone();
    ctx.spawn(
      stream
        .collect()
        .into_future()
        .and_then(move |reqs| {
          let mut maybe_resource_name = None;
          let mut want_next_offset = 0;
          let mut bytes = Bytes::new();
          for req in reqs {
            match maybe_resource_name {
              None => maybe_resource_name = Some(req.get_resource_name().to_owned()),
              Some(ref resource_name) => {
                if resource_name != req.get_resource_name() {
                  return Err(grpcio::Error::RpcFailure(grpcio::RpcStatus::new(
                    grpcio::RpcStatusCode::InvalidArgument,
                    Some(format!(
                      "All resource names in stream must be the same. Got {} but earlier saw {}",
                      req.get_resource_name(),
                      resource_name
                    )),
                  )));
                }
              }
            }
            if req.get_write_offset() != want_next_offset {
              return Err(grpcio::Error::RpcFailure(grpcio::RpcStatus::new(
                grpcio::RpcStatusCode::InvalidArgument,
                Some(format!(
                  "Missing chunk. Expected next offset {}, got next offset: {}",
                  want_next_offset,
                  req.get_write_offset()
                )),
              )));
            }
            want_next_offset += req.get_data().len() as i64;
            write_message_sizes
              .lock()
              .unwrap()
              .push(req.get_data().len());
            bytes.extend(req.get_data());
          }
          Ok((maybe_resource_name, bytes))
        })
        .map_err(move |err: grpcio::Error| match err {
          grpcio::Error::RpcFailure(status) => status,
          e => grpcio::RpcStatus::new(grpcio::RpcStatusCode::Unknown, Some(format!("{:?}", e))),
        })
        .and_then(
          move |(maybe_resource_name, bytes)| match maybe_resource_name {
            None => Err(grpcio::RpcStatus::new(
              grpcio::RpcStatusCode::InvalidArgument,
              Some("Stream saw no messages".to_owned()),
            )),
            Some(resource_name) => {
              let parts: Vec<_> = resource_name.splitn(6, '/').collect();
              if parts.len() != 6
                || parts.get(1) != Some(&"uploads")
                || parts.get(3) != Some(&"blobs")
              {
                return Err(grpcio::RpcStatus::new(
                  grpcio::RpcStatusCode::InvalidArgument,
                  Some(format!("Bad resource name: {}", resource_name)),
                ));
              }
              let fingerprint = match Fingerprint::from_hex_string(parts[4]) {
                Ok(f) => f,
                Err(err) => {
                  return Err(grpcio::RpcStatus::new(
                    grpcio::RpcStatusCode::InvalidArgument,
                    Some(format!(
                      "Bad fingerprint in resource name: {}: {}",
                      parts[4], err
                    )),
                  ))
                }
              };
              let size = match parts[5].parse::<usize>() {
                Ok(s) => s,
                Err(err) => {
                  return Err(grpcio::RpcStatus::new(
                    grpcio::RpcStatusCode::InvalidArgument,
                    Some(format!("Bad size in resource name: {}: {}", parts[5], err)),
                  ))
                }
              };
              if size != bytes.len() {
                return Err(grpcio::RpcStatus::new(
                  grpcio::RpcStatusCode::InvalidArgument,
                  Some(format!(
                    "Size was incorrect: resource name said size={} but got {}",
                    size,
                    bytes.len()
                  )),
                ));
              }

              if should_always_fail {
                return Err(grpcio::RpcStatus::new(
                  grpcio::RpcStatusCode::Internal,
                  Some("StubCAS is configured to always fail".to_owned()),
                ));
              }

              {
                let mut blobs = blobs.lock().unwrap();
                blobs.insert(fingerprint, bytes);
              }

              let mut response = bazel_protos::bytestream::WriteResponse::new();
              response.set_committed_size(size as i64);
              Ok(response)
            }
          },
        )
        .then(move |result| match result {
          Ok(resp) => sink.success(resp),
          Err(err) => sink.fail(err),
        })
        .then(move |_| Ok(())),
    );
  }

  fn query_write_status(
    &self,
    _ctx: grpcio::RpcContext,
    _req: bazel_protos::bytestream::QueryWriteStatusRequest,
    sink: grpcio::UnarySink<bazel_protos::bytestream::QueryWriteStatusResponse>,
  ) {
    sink.fail(grpcio::RpcStatus::new(
      grpcio::RpcStatusCode::Unimplemented,
      None,
    ));
  }
}

impl bazel_protos::remote_execution_grpc::ContentAddressableStorage for StubCASResponder {
  fn find_missing_blobs(
    &self,
    _ctx: grpcio::RpcContext,
    req: bazel_protos::remote_execution::FindMissingBlobsRequest,
    sink: grpcio::UnarySink<bazel_protos::remote_execution::FindMissingBlobsResponse>,
  ) {
    if self.should_always_fail() {
      sink.fail(grpcio::RpcStatus::new(
        grpcio::RpcStatusCode::Internal,
        Some("StubCAS is configured to always fail".to_owned()),
      ));
      return;
    }
    let blobs = self.blobs.lock().unwrap();
    let mut response = bazel_protos::remote_execution::FindMissingBlobsResponse::new();
    for digest in req.get_blob_digests() {
      let hashing_digest_result: Result<Digest, String> = digest.into();
      let hashing_digest = hashing_digest_result.expect("Bad digest");
      if !blobs.contains_key(&hashing_digest.0) {
        response.mut_missing_blob_digests().push(digest.clone())
      }
    }
    sink.success(response);
  }

  fn batch_update_blobs(
    &self,
    _ctx: grpcio::RpcContext,
    _req: bazel_protos::remote_execution::BatchUpdateBlobsRequest,
    sink: grpcio::UnarySink<bazel_protos::remote_execution::BatchUpdateBlobsResponse>,
  ) {
    sink.fail(grpcio::RpcStatus::new(
      grpcio::RpcStatusCode::Unimplemented,
      None,
    ));
  }
  fn get_tree(
    &self,
    _ctx: grpcio::RpcContext,
    _req: bazel_protos::remote_execution::GetTreeRequest,
    _sink: grpcio::ServerStreamingSink<bazel_protos::remote_execution::GetTreeResponse>,
  ) {
    // Our client doesn't currently use get_tree, so we don't bother implementing it.
    // We will need to if the client starts wanting to use it.
    unimplemented!()
  }
}
