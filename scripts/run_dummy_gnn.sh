cd ..
cd data-processing
shopt -s globstar
python dummy_gnn.py \
  --input_files ../data/triples-gnn/**/**/*.json \
  --output_file ../data/results/dummy-gnn/simple-fnn.json