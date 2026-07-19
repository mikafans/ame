import os

import httpx
import pytest


@pytest.fixture(scope="session")
def client():
    with httpx.Client(
        base_url=os.getenv("AME_API_URL", "http://localhost:28800"),
        timeout=30.0,
    ) as value:
        yield value
