echo "Running Dummy Models"
cd ../data-processing
python dummy.py -i ../data/triples/**/*.json -o ../data/results/dummy
python dummy.py -i ../data/triples-gnn/**/*.json -o ../data/results/dummy-gnn --gnn

echo "Computing Statistics"
cd ../pipeline
./target/release/pipeline summarise-triple-performance \
  -i \
    ../data/results/dummy/dummy.json \
    ../data/results/dummy-gnn/dummy-negative.json \
    ../data/results/dummy-gnn/dummy-positive.json \
    ../data/results/dummy-gnn/dummy-random.json \
    ../data/results/dummy-gnn/dummy-weighted-random.json \
  -o ../data/results-temp

echo "Building Figures"
cd ../data-processing
python perf-to-paper.py \
  --files \
    ../data/results-temp/dummy \
    ../data/results-temp/dummy-negative \
    ../data/results-temp/dummy-positive \
    ../data/results-temp/dummy-random \
    ../data/results-temp/dummy-weighted-random \
  --metrics accuracy "f1" \
  --name_mapping "dummy=Dummy,dummy-negative=Dummy (negative),dummy-positive=Dummy (positive),dummy-random=Dummy (random),dummy-weighted-random=Dummy (weighted random)"
