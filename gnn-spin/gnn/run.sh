python ml.py \
  --structure_directory ../../data/prepared \
  --embedding_directory ../../data/embeddings/output/compacted-projects \
  --graph_directory ../../data/graphs \
  --model_config gcn.yaml \
  --output_directory ../../data/gnn-output \
  --exclude_projects apache-derby apache-cxf \
  --project_legacy_mapping "hibernate=hibernate-core" \
  --model dummy