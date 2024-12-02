cd ..
cd data-processing
shopt -s globstar
python gnn.py \
  --input_files ../data/triples-gnn/apache-ant/**/*.json \
  --source_directory ../data/compacted-projects \
  --embedding_directory ../data/embeddings/output/compacted-projects \
  --output_file ../data/results/gnn/simple-gnn.json
