# Copyright 2015 Pants project contributors (see CONTRIBUTORS.md).
# Licensed under the Apache License, Version 2.0 (see LICENSE).


python_tests(
  name='cloc',
  sources=['test_cloc.py'],
  coverage=['pants.backend.graph_info.tasks.cloc'],
  dependencies=[
    '3rdparty/python:future',
    'src/python/pants/backend/graph_info/tasks',
    'src/python/pants/backend/jvm/targets:java',
    'src/python/pants/backend/python/targets:python',
    'src/python/pants/base:build_environment',
    'tests/python/pants_test:task_test_base',
  ]
)

python_tests(
  name='cloc_integration',
  sources=['test_cloc_integration.py'],
  dependencies=[
    'tests/python/pants_test:int-test',
  ],
  tags = {'integration'},
)

python_tests(
  name = 'dependees',
  sources = ['test_dependees.py'],
  dependencies = [
    'src/python/pants/backend/codegen/thrift/java',
    'src/python/pants/backend/graph_info/tasks',
    'src/python/pants/java/jar',
    'src/python/pants/backend/jvm/targets:java',
    'src/python/pants/backend/jvm/targets:jvm',
    'src/python/pants/backend/python/targets:python',
    'src/python/pants/build_graph',
    'tests/python/pants_test:task_test_base',
  ]
)

python_tests(
  name = 'filemap',
  sources = ['test_filemap.py'],
  dependencies = [
    'src/python/pants/backend/graph_info/tasks',
    'src/python/pants/backend/python/targets:python',
    'src/python/pants/build_graph',
    'tests/python/pants_test:task_test_base',
  ],
)

python_tests(
  name = 'filter',
  sources = ['test_filter.py'],
  dependencies = [
    'src/python/pants/backend/docgen/targets',
    'src/python/pants/backend/graph_info/tasks',
    'src/python/pants/backend/jvm/targets:java',
    'src/python/pants/backend/python/targets:python',
    'src/python/pants/base:exceptions',
    'src/python/pants/build_graph',
    'tests/python/pants_test:task_test_base',
  ],
)

python_tests(
  name = 'listtargets',
  sources = ['test_listtargets.py'],
  dependencies = [
    '3rdparty/python:future',
    'src/python/pants/backend/graph_info/tasks',
    'src/python/pants/backend/jvm:artifact',
    'src/python/pants/backend/jvm/targets:java',
    'src/python/pants/backend/jvm/targets:jvm',
    'src/python/pants/build_graph',
    'src/python/pants/backend/python/targets:python',
    'tests/python/pants_test/subsystem:subsystem_utils',
    'tests/python/pants_test:task_test_base',
  ],
)

python_tests(
  name = 'minimal_cover',
  sources = ['test_minimal_cover.py'],
  dependencies = [
    'src/python/pants/backend/graph_info/tasks',
    'src/python/pants/backend/python/targets:python',
    'src/python/pants/build_graph',
    'tests/python/pants_test:task_test_base',
  ],
)

python_tests(
  name='paths',
  sources=['test_paths.py'],
  dependencies=[
    '3rdparty/python:future',
    'src/python/pants/backend/graph_info/tasks',
    'src/python/pants/base:exceptions',
    'tests/python/pants_test:task_test_base',
  ]
)

python_tests(
  name='paths_integration',
  sources=['test_paths_integration.py'],
  dependencies=[
    'tests/python/pants_test:int-test',
  ],
  tags = {'integration'},
)

python_tests(
  name = 'sorttargets',
  sources = ['test_sorttargets.py'],
  dependencies = [
    'src/python/pants/backend/graph_info/tasks',
    'src/python/pants/backend/python/targets:python',
    'src/python/pants/build_graph',
    'tests/python/pants_test:task_test_base',
  ],
)

python_tests(
  name='target_filter_task_mixin',
  sources=['test_target_filter_task_mixin.py'],
  dependencies=[
    'src/python/pants/backend/graph_info/tasks',
    'src/python/pants/build_graph',
    'tests/python/pants_test:task_test_base',
  ]
)
