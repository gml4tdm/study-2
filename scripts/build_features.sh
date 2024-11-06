cd ..
cd pipeline
cargo build --release

if [ "$1" != "skip-graphs" ]; then
  ./target/release/pipeline generate-train-test-triples \
    -i ../data/graphs/apache-ant/*.odem \
    -o ../data/triples/apache-ant \
    --language java \
    --only-common-nodes-for-training
  ./target/release/pipeline generate-train-test-triples \
    -i ../data/graphs/apache-camel/*.odem \
    -o ../data/triples/apache-camel \
    --language java \
    --only-common-nodes-for-training
  ./target/release/pipeline generate-train-test-triples \
    -i ../data/graphs/apache-cxf/*.odem \
    -o ../data/triples/apache-cxf \
    --language java \
    --only-common-nodes-for-training
  ./target/release/pipeline generate-train-test-triples \
    -i ../data/graphs/apache-derby/*.odem \
    -o ../data/triples/apache-derby \
    -m "db-derby=apache-derby" \
    --language java \
    --only-common-nodes-for-training
  ./target/release/pipeline generate-train-test-triples \
    -i ../data/graphs/hibernate/*.odem \
    -o ../data/triples/hibernate \
    -m "hibernate-core=hibernate" \
    --language java \
    --only-common-nodes-for-training
fi

exit
shopt -s globstar
#./target/release/pipeline add-source-information-to-triples \
#  -i ../data/triples/apache-ant/apache-ant-1.1-1.2-1.3.json \
#  -o ../data/triples-with-source/ \
#  -s ../data/compacted-projects

./target/release/pipeline add-source-information-to-triples \
  -i ../data/triples/**/*.json \
  -o ../data/triples-with-source/ \
  -s ../data/compacted-projects
