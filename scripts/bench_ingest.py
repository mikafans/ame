import sys
import uuid
import time
import requests
import random

API_URL = "http://localhost:28080"
BATCH_SIZE = 50
NUM_BATCHES = 20
NUM_TAGS = 1000

def get_token():
    resp = requests.post(f"{API_URL}/v1/auth/login", json={"email": "admin@example.com", "password": "password123"})
    if resp.status_code != 200:
        print(f"Login failed: {resp.text}")
        sys.exit(1)
    return resp.json()["token"]

def generate_tags(n):
    return [f"tag_{i}_{uuid.uuid4().hex[:4]}" for i in range(n)]

def generate_batch(tags):
    batch = []
    for _ in range(BATCH_SIZE):
        batch.append({
            "kind": "mc",
            "prompt": f"Benchmark question {uuid.uuid4()}",
            "payload": {"options": ["A", "B", "C", "D"], "correct_index": 0},
            "points": 1,
            "tags": random.sample(tags, 5)
        })
    return batch

def run_bench():
    token = get_token()
    headers = {"Authorization": f"Bearer {token}"}
    
    tags = generate_tags(NUM_TAGS)
    
    print(f"Ingesting {NUM_BATCHES * BATCH_SIZE} questions...")
    start = time.perf_counter()
    
    for i in range(NUM_BATCHES):
        batch = generate_batch(tags)
        resp = requests.post(f"{API_URL}/v1/questions", json={"questions": batch}, headers=headers)
        if resp.status_code != 201:
            print(f"Batch {i} failed: {resp.status_code} {resp.text}")
            sys.exit(1)
            
    end = time.perf_counter()
    print(f"Total time: {end - start:.2f}s")
    print(f"Questions/sec: {(NUM_BATCHES * BATCH_SIZE) / (end - start):.2f}")

if __name__ == "__main__":
    run_bench()
