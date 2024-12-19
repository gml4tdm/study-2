cd ..data-processing
python svm_time.py \
  -t ../data/svm-time-triples/**/*.json \
  -g ../data/svm-time-graph-changes/*.json \
  -c ../data/svm-time-co-changes/*.json \
  -m ../data/graphs/**/*.json \
  -o ../data/svm/svm-time-results.json