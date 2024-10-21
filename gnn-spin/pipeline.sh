if ! command -v cargo version 2>&1> /dev/null then
  echo "<cargo> could not be found!"
  exit 1
fi

if [ ! -d "../../data/graphs" ]; then
  echo "Directory ../../data/graphs does not exist."
  echo "Please run replication/prepare_data.py first."
  exit 1
fi

echo "Downloading source code"
cd downloader
cargo run --release -- ../../data/versions.json ../../data/source-code
cd ..

echo "Compacting source code"
cd compacter
cargo run --release -- \
  ../../data/source-code \
  ../../data/graphs \
  ../../data/compacted-projects
cd ..

echo "Deleting source code to save disk space"
rm -rf ../../data/soruce code

echo "Extracting file structure information"
cd structure
cargo run --release -- ../../data/compacted-projects ../../data/structure
cd ..
