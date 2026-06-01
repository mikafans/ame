import time
import requests

API_URL = "http://localhost:28080"

def run_bench():
    # Login as admin to get token
    resp = requests.post(f"{API_URL}/v1/auth/login", json={"email": "admin@example.com", "password": "password123"})
    token = resp.json()["token"]
    headers = {"Authorization": f"Bearer {token}"}
    
    print("Benchmarking question query...")
    start = time.perf_counter()
    
    # Query questions with tag filter
    resp = requests.get(f"{API_URL}/v1/questions", params={"tag": "tag_0_0"}, headers=headers)
    
    end = time.perf_counter()
    print(f"Total time for query: {end - start:.4f}s")

if __name__ == "__main__":
    run_bench()
