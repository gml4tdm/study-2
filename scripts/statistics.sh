SKIP_RUST=false
SKIP_STATS=false
SKIP_GRAPHS=false

SKIP_EVOLUTION=false
SKIP_TRIPLES=false
SKIP_GLOBAL=false

for arg in "$@"; do
  case $arg in
    --skip-rust)
      SKIP_RUST=true
      ;;
    --skip-stats)
      SKIP_STATS=true
      ;;
    --skip-graphs)
      SKIP_GRAPHS=true
      ;;
    --skip-evolution)
      SKIP_EVOLUTION=true
      ;;
    --skip-triples)
      SKIP_TRIPLES=true
      ;;
    --skip-global)
      SKIP_GLOBAL=true
      ;;
  esac
done

echo "SKIP_RUST=$SKIP_RUST"
echo "SKIP_STATS=$SKIP_STATS"
echo "SKIP_GRAPHS=$SKIP_GRAPHS"
echo "SKIP_EVOLUTION=$SKIP_EVOLUTION"
echo "SKIP_TRIPLES=$SKIP_TRIPLES"
echo "SKIP_GLOBAL=$SKIP_GLOBAL"

cd ..
cd pipeline
if [ "$SKIP_RUST" = false ]; then
  cargo build --release
else
  echo "Skipping rust build"
fi

shopt -s globstar

if [ "$SKIP_STATS" = false ]; then
  ./target/release/pipeline compute-project-evolution-statistics \
      --files ../data/graphs/apache-ant/*.odem \
      --output ../data/statistics/apache-ant.json \
      --package-graph
  ./target/release/pipeline compute-project-evolution-statistics \
      --files ../data/graphs/apache-camel/*.odem \
      --output ../data/statistics/apache-camel.json \
      --package-graph
  ./target/release/pipeline compute-project-evolution-statistics \
      --files ../data/graphs/apache-derby/*.odem \
      --output ../data/statistics/apache-derby.json \
      --package-graph
  ./target/release/pipeline compute-project-evolution-statistics \
      --files ../data/graphs/apache-cxf/*.odem \
      --output ../data/statistics/apache-cxf.json \
      --package-graph
  ./target/release/pipeline compute-project-evolution-statistics \
      --files ../data/graphs/hibernate/*.odem \
      --output ../data/statistics/hibernate.json \
      --package-graph
else
  echo "Skipping statistics"
fi

if [ "$SKIP_GRAPHS" = false ]; then
  ./target/release/pipeline generate-train-test-triples \
    -i ../data/graphs/apache-ant/*.odem \
    -o ../data/triples/apache-ant \
    --language java
  ./target/release/pipeline generate-train-test-triples \
    -i ../data/graphs/apache-camel/*.odem \
    -o ../data/triples/apache-camel \
    --language java
  ./target/release/pipeline generate-train-test-triples \
    -i ../data/graphs/apache-cxf/*.odem \
    -o ../data/triples/apache-cxf \
    --language java
  ./target/release/pipeline generate-train-test-triples \
    -i ../data/graphs/apache-derby/*.odem \
    -o ../data/triples/apache-derby \
    -m "db-derby=apache-derby" \
    --language java
  ./target/release/pipeline generate-train-test-triples \
    -i ../data/graphs/hibernate/*.odem \
    -o ../data/triples/hibernate \
    -m "hibernate-core=hibernate" \
    --language java
else
  echo "Skipping graphs"
fi

echo "Generating figures"

cd ..
cd data-processing

if [ "$SKIP_EVOLUTION" = false ]; then
  python project_evolution_statistics.py \
    -i ../data/statistics/*.json \
    -o ../data/figures
else
  echo "Skipping evolution statistics"
fi

if [ "$SKIP_TRIPLES" = false ]; then
  for filename in ../data/triples/*; do
    python triple_statistics.py -i $filename/*.json -o ../data/figures
  done
else
  echo "Skipping triple statistics"
fi

if [ "$SKIP_GLOBAL" = false ]; then
  python global_triple_statistics.py -i ../data/triples/**/*.json -o ../data/figures/
else
  echo "Skipping global triple statistics"
fi

echo "Done"
