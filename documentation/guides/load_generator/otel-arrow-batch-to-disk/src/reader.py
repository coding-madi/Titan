import sys
import pyarrow.ipc as ipc

if len(sys.argv) < 2:
    print("Usage: python reader.py <arrow_file_path>")
    sys.exit(1)

FILENAME = sys.argv[1]

with open(FILENAME, "rb") as f:
    reader = ipc.RecordBatchFileReader(f)
    table = reader.read_all()

df = table.to_pandas()
print(df)