#echo "Running Dummy Models"
#cd ../data-processing
#python dummy.py -i ../data/triples/**/*.json -o ../data/results/dummy
#python dummy.py -i ../data/triples-gnn/**/*.json -o ../data/results/dummy-gnn --gnn

echo "Computing Statistics"
cd ../pipeline
./target/release/pipeline summarise-triple-performance \
  -i \
    ../data/results/svm/replication.json \
    ../data/results/dummy/dummy.json \
    ../data/results/tommasel/aspredictor.json \
    ../data/results/svm/initial_time_svm_results.json \
  -o ../data/results-temp

echo "Building Figures"
cd ../data-processing
python perf-to-paper.py \
  --files \
    ../data/results-temp/dummy \
    ../data/results-temp/replication \
    ../data/results-temp/aspredictor \
    ../data/results-temp/initial_time_svm_results \
  --name_mapping "dummy=Dummy,replication=SVM (ours),aspredictor=SVM (Tommasel),initial_time_svm_results=SVM (Replication + Historical)" \
  --metrics \
    accuracy \
    "f1" \
    precision \
    recall \
    normalised_true_positives \
    normalised_false_positives \
    normalised_false_negatives \
    normalised_true_positives \
