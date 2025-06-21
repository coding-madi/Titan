# OTEL Contract Log Generation (Apache Arrow Format)

## 1. Overview
This project demonstrates the generation, batching, and storage of OpenTelemetry (OTEL) logs using the Apache Arrow columnar format. Logs are written to Arrow files and a continuous stream file, enabling high-performance analytics, efficient storage, and easy integration with modern data processing tools. The log schema is defined according to the OTEL standard using Arrow's schema system.

## 2. Directory Structure
```
otel-arrow-batch-to-disk/
├── README.md
├── requirements.txt
├── docs/
│   ├── logGenFlow.png
│   └── logGenFlow.puml
├── files/
│   ├── otel_logs_YYYYMMDD_HHMM.arrow
│   └── otel_logs.stream
└── src/
    ├── generator.py
    ├── main.py
    ├── reader.py
    ├── schema.py
    └── writer.py
```

## 3. Sequence Diagram

![Sequence Diagram](docs/logGenFlow.png)

PUML source: [`docs/sequence.puml`](docs/logGenFlow.puml)

## 4. Getting Started

### Prerequisites
- Python 3.8 or higher
- Install dependencies:
  ```bash
  pip install -r requirements.txt
  ```

### Generating Logs
To generate OTEL logs and write them in Arrow format:
```bash
python src/main.py
```
- Generates 3 OTEL logs per second.
- Batches and writes logs to Arrow and stream files every 60 seconds.
- Output files (`.arrow` and `.stream`) are saved in the `files/` directory.

### Reading Arrow Files
To read and display the contents of an Arrow file:
```bash
python src/reader.py files/otel_logs_YYYYMMDD_HHMM.arrow
```
- Replace `files/otel_logs_YYYYMMDD_HHMM.arrow` with the actual file name.
- The script prints the Arrow file contents as a pandas DataFrame in the terminal.

## 5. Example Output

**main.py output:**
```
Generating 3 OTEL logs/sec — batching every 60s and rotating files...
Writing 180 records...
Wrote batch to files/otel_logs_20250621_1058.arrow
Appended 180 records to files/otel_logs.stream
```

**reader.py output:**
```
          time_unix_nano  severity_number  ...  span_id  attributes
0    1750503437164505088               13  ...  ...      ...
1    ...
...
[180 rows x 7 columns]
```

---
For questions, issues, or customization requests, please open an issue or contact the project maintainer.
