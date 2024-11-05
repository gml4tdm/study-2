cd ..
cd pipeline
cargo build --release

./target/release/pipeline generate-train-test-triples -i ../data/graphs/apache-ant/*.odem -o ../data/triples/apache-ant
./target/release/pipeline generate-train-test-triples -i ../data/graphs/apache-camel/*.odem -o ../data/triples/apache-camel
./target/release/pipeline generate-train-test-triples -i ../data/graphs/apache-cxf/*.odem -o ../data/triples/apache-cxf
./target/release/pipeline generate-train-test-triples -i ../data/graphs/apache-derby/*.odem -o ../data/triples/apache-derby
./target/release/pipeline generate-train-test-triples -i ../data/graphs/hibernate/*.odem -o ../data/triples/hibernate