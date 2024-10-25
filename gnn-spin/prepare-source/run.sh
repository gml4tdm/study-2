cargo run --release -- \
  --graph-directory ../../data/graphs \
  --source-directory ../../data/sources \
  --output-directory ../../data/prepared \
  --graph-format odem \
  --project-name-mapping "hibernate=hibernate-core;apache-derby=db-derby"