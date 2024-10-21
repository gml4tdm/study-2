python __main__.py \
  --structure_directory ../../data/structure \
  --embedding_directory ../../data/embeddings \
  --graph_directory ../../data/graphs \
  --model_config gcn.yaml \
  --output_directory ../../data/gnn-output \
  --exclude_projects apache-derby apache-ant apache-cxf hibernate \
  --project_legacy_mapping "hibernate=hibernate-core"