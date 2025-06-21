# generator.py
import uuid
import random
from datetime import datetime, timezone

def generate_log():
    now = datetime.now(timezone.utc)
    severity_number = random.choice([9, 13, 17])
    text_map = {9: "INFO", 13: "WARN", 17: "ERROR"}
    return {
        "time_unix_nano": int(now.timestamp() * 1e9),
        "severity_number": severity_number,
        "severity_text": text_map[severity_number],
        "body": random.choice(["Login ok", "Disk warning", "Started", "DB timeout"]),
        "trace_id": uuid.uuid4().bytes,
        "span_id": uuid.uuid4().bytes[:8],
        "attributes": [
            ("service.name", "auth-service"),
            ("env", "prod"),
            ("region", "us-west-2")
        ]
    }