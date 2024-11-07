cd ..
cd pipeline
cargo build --release

shopt -s globstar

./target/release/pipeline graphs-to-dot \
  -i ../data/graphs/apache-ant/apache-ant-1.1.odem \
  -o ../data/dot \
  --package-diagrams
./target/release/pipeline graphs-to-dot \
  -i ../data/graphs/apache-ant/apache-ant-1.2.odem \
  -o ../data/dot \
  --package-diagrams
./target/release/pipeline graphs-to-dot \
  -i ../data/graphs/apache-ant/apache-ant-1.3.odem \
  -o ../data/dot \
  --package-diagrams

cd ../data/dot

for file in *.dot; do
  echo "Rendering $file"
  dot -Tpng $file -o ${file%.*}.png
done