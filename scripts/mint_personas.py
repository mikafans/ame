# /// script
# requires-python = ">=3.11"
# dependencies = ["httpx>=0.27", "rich>=13"]
# ///
"""
Mint distinct persona-based assessments for Ada and Mira using the agent surface tools.
"""

import argparse
import os
import sys
from typing import Any

import httpx
from rich.console import Console

console = Console()

DEFAULT_API = os.environ.get("AME_API_URL", "http://localhost:28080")

ADA_QUESTIONS = [
    {
        "kind": "mc",
        "prompt": "What is the average-case time complexity of insertion sort on an array?",
        "payload": {
            "options": ["O(n log n)", "O(n)", "O(n²)", "O(1)"],
            "correct_index": 2,
        },
        "explanation": "Insertion sort takes O(n²) average time due to nested comparisons and shifts.",
        "tags": ["sorting", "algorithms"],
        "points": 1,
    },
    {
        "kind": "mc",
        "prompt": "Which binary tree traversal visits nodes in-order (Left, Root, Right)?",
        "payload": {
            "options": ["Pre-order", "In-order", "Post-order", "Level-order"],
            "correct_index": 1,
        },
        "explanation": "In-order traversal visits left subtree, then the root, then the right subtree.",
        "tags": ["data-structures"],
        "points": 1,
    },
    {
        "kind": "tf",
        "prompt": "Dijkstra's algorithm can correctly handle graphs with negative edge weights.",
        "payload": {"correct": False},
        "explanation": "Dijkstra's algorithm assumes non-negative edge weights. Bellman-Ford should be used instead.",
        "tags": ["graphs", "algorithms"],
        "points": 1,
    },
    {
        "kind": "short",
        "prompt": "Name the graph algorithm that finds a minimum spanning tree by sorting edges.",
        "payload": {
            "accepted": ["kruskal", "kruskal's", "Kruskal"],
            "normalize": "case_insensitive_strip_accents",
            "judge": "exact",
        },
        "explanation": "Kruskal's algorithm sorts edges by weight and adds them if they do not form a cycle.",
        "tags": ["graphs"],
        "points": 1,
    },
    {
        "kind": "mc",
        "prompt": "What is the primary benefit of dynamic programming over naive recursion?",
        "payload": {
            "options": ["Lower space complexity", "Avoids repeating calculations of overlapping subproblems", "Easier implementation", "Guarantees exact solutions"],
            "correct_index": 1,
        },
        "explanation": "Dynamic programming caches overlapping subproblem solutions to prevent redundant recursion.",
        "tags": ["dynamic-programming"],
        "points": 1,
    },
    {
        "kind": "tf",
        "prompt": "A stack is a Last-In, First-Out (LIFO) data structure.",
        "payload": {"correct": True},
        "explanation": "Items inserted last into a stack are popped first.",
        "tags": ["data-structures"],
        "points": 1,
    },
    {
        "kind": "short",
        "prompt": "What keyword is used to define a function in Python?",
        "payload": {
            "accepted": ["def"],
            "normalize": "exact",
            "judge": "exact",
        },
        "explanation": "The 'def' keyword introduces a function definition in Python.",
        "tags": ["python"],
        "points": 1,
    },
    {
        "kind": "mc",
        "prompt": "Which algorithm is a divide-and-conquer sorting algorithm?",
        "payload": {
            "options": ["Bubble Sort", "Insertion Sort", "Selection Sort", "Merge Sort"],
            "correct_index": 3,
        },
        "explanation": "Merge sort divides the array, sorts the halves recursively, and merges them.",
        "tags": ["sorting"],
        "points": 1,
    },
    {
        "kind": "tf",
        "prompt": "The set data structure in Python allows duplicate elements.",
        "payload": {"correct": False},
        "explanation": "A Python set only stores unique elements; duplicates are ignored.",
        "tags": ["python"],
        "points": 1,
    },
    {
        "kind": "mc",
        "prompt": "What is the sum of integers from 1 to 10?",
        "payload": {
            "options": ["45", "50", "55", "60"],
            "correct_index": 2,
        },
        "explanation": "The sum is given by n*(n+1)//2 = 10*11//2 = 55.",
        "tags": ["math"],
        "points": 1,
    }
]

MIRA_QUESTIONS = [
    {
        "kind": "mc",
        "prompt": "What interval is formed between the pitches C and E?",
        "payload": {
            "options": ["Minor third", "Major third", "Perfect fourth", "Minor second"],
            "correct_index": 1,
        },
        "explanation": "C to E is a major third, consisting of 4 semitones.",
        "tags": ["intervals", "music-theory"],
        "points": 1,
    },
    {
        "kind": "mc",
        "prompt": "Which major key signature contains exactly one sharp (F#)?",
        "payload": {
            "options": ["C major", "D major", "G major", "F major"],
            "correct_index": 2,
        },
        "explanation": "G major contains exactly one sharp: F#.",
        "tags": ["scales", "music-theory"],
        "points": 1,
    },
    {
        "kind": "tf",
        "prompt": "The relative major of A minor is C major.",
        "payload": {"correct": True},
        "explanation": "C major and A minor share the same key signature (no sharps or flats).",
        "tags": ["scales", "music-theory"],
        "points": 1,
    },
    {
        "kind": "short",
        "prompt": "Identify the pitch that is a perfect fifth above A.",
        "payload": {
            "accepted": ["E", "e"],
            "normalize": "case_insensitive_strip_accents",
            "judge": "exact",
        },
        "explanation": "A perfect fifth above A is E (7 semitones).",
        "tags": ["intervals"],
        "points": 1,
    },
    {
        "kind": "mc",
        "prompt": "What texture is formed by multiple independent musical voices intertwining?",
        "payload": {
            "options": ["Monophonic", "Homophonic", "Polyphonic", "Heterophonic"],
            "correct_index": 2,
        },
        "explanation": "Polyphony consists of independent melodic voices sounding together.",
        "tags": ["harmony", "music-theory"],
        "points": 1,
    },
    {
        "kind": "tf",
        "prompt": "A major triad is composed of a major third and a perfect fifth above the root.",
        "payload": {"correct": True},
        "explanation": "Major triad formula: Root + Major Third + Perfect Fifth.",
        "tags": ["harmony"],
        "points": 1,
    },
    {
        "kind": "short",
        "prompt": "Identify the relative minor key of F major.",
        "payload": {
            "accepted": ["d minor", "D minor", "d-minor", "D-minor"],
            "normalize": "case_insensitive_strip_accents",
            "judge": "exact",
        },
        "explanation": "The relative minor of F major is D minor (both share B flat).",
        "tags": ["scales"],
        "points": 1,
    },
    {
        "kind": "mc",
        "prompt": "Which musical interval is famously known as the tritone?",
        "payload": {
            "options": ["Perfect fourth", "Augmented fourth", "Minor fifth", "Major third"],
            "correct_index": 1,
        },
        "explanation": "An augmented fourth (or diminished fifth) spans exactly 3 whole tones (tritone).",
        "tags": ["intervals"],
        "points": 1,
    },
    {
        "kind": "tf",
        "prompt": "A minor triad is composed of a minor third and a diminished fifth above the root.",
        "payload": {"correct": False},
        "explanation": "A minor triad consists of a minor third and a perfect fifth (not diminished).",
        "tags": ["harmony"],
        "points": 1,
    },
    {
        "kind": "mc",
        "prompt": "What does the top number of a time signature indicate?",
        "payload": {
            "options": ["The value of each beat", "The speed of the piece", "The number of beats in each measure", "The overall key signature"],
            "correct_index": 2,
        },
        "explanation": "The top number represents how many beats are in a single bar.",
        "tags": ["music-theory", "ear-training"],
        "points": 1,
    }
]

def mint_persona(client, email, password, agent_label, focus_tags, title, description, course, questions):
    console.print(f"\n[bold cyan]Minting for {email}...[/bold cyan]")
    # 1. Login
    resp = client.post("/v1/auth/login", json={"email": email, "password": password})
    if not resp.is_success:
        console.print(f"[red]Failed to log in as {email}[/red]")
        return
    token = resp.json()["token"]
    headers = {"Authorization": f"Bearer {token}"}

    # 2. Check and get/create agent
    agent_resp = client.post("/v1/me/agents", json={
        "label": agent_label,
        "scopes": ["assessment.read", "assessment.write"],
        "focusTags": focus_tags
    }, headers=headers)
    
    if agent_resp.status_code == 429:
        # Fetch existing agents
        list_resp = client.get("/v1/me/agents", headers=headers)
        if list_resp.is_success:
            for a in list_resp.json().get("agents", []):
                client.delete(f"/v1/me/agents/{a['id']}", headers=headers)
            # Retry
            agent_resp = client.post("/v1/me/agents", json={
                "label": agent_label,
                "scopes": ["assessment.read", "assessment.write"],
                "focusTags": focus_tags
            }, headers=headers)
            
    if not agent_resp.is_success:
        console.print(f"[red]Failed to create/retrieve agent for {email}: {agent_resp.text}[/red]")
        return
        
    agent_data = agent_resp.json()
    agent_key = agent_data["apiKey"]
    agent_headers = {"Authorization": f"Bearer {agent_key}"}
    console.print(f"  [green]✓[/green] Agent active: {agent_data['id']}")

    # 3. Mint assessment via agent tools
    run_resp = client.post("/v1/agents/run", json={
        "tool": "assessment.create",
        "params": {
            "title": title,
            "description": description,
            "mode": "practice",
            "objectives": [f"Understand {t.replace('-', ' ')}" for t in focus_tags[:3]],
            "course": course,
            "method": "agent",
            "questions": questions
        }
    }, headers=agent_headers)

    if not run_resp.is_success or not run_resp.json().get("ok"):
        console.print(f"  [red]✗[/red] Failed to mint assessment: {run_resp.text}")
        return
        
    res_data = run_resp.json()["result"]
    assessment_id = res_data["id"]
    console.print(f"  [green]✓[/green] Minted assessment '{title}' ({assessment_id[:8]}…) with {len(questions)} questions")

    # 4. Publish assessment
    pub_resp = client.post("/v1/agents/run", json={
        "tool": "assessment.update",
        "params": {
            "id": assessment_id,
            "status": "active"
        }
    }, headers=agent_headers)

    if pub_resp.is_success and pub_resp.json().get("ok"):
        console.print("  [green]✓[/green] Assessment published successfully")
    else:
        console.print(f"  [red]✗[/red] Failed to publish assessment: {pub_resp.text}")

def main():
    parser = argparse.ArgumentParser(description="Mint custom persona content for Ada and Mira.")
    parser.add_argument("--api", default=DEFAULT_API)
    args = parser.parse_args()

    with httpx.Client(base_url=args.api, timeout=30.0) as client:
        # Mint for Ada
        mint_persona(
            client=client,
            email="ada@example.com",
            password="password123",
            agent_label="Ada's Computer Science Assistant",
            focus_tags=["algorithms", "data-structures", "sorting", "graphs", "dynamic-programming"],
            title="Advanced Algorithms & Data Structures Practice",
            description="Exhaustive evaluation of sorting, graph logic, and algorithmic efficiency.",
            course="Computer Science II",
            questions=ADA_QUESTIONS
        )

        # Mint for Mira
        mint_persona(
            client=client,
            email="mira@example.com",
            password="password123",
            agent_label="Mira's Music Theory Assistant",
            focus_tags=["music-theory", "harmony", "scales", "intervals", "ear-training"],
            title="Introduction to Music Theory",
            description="Fundamentals of notation, interval recognition, scale degrees, and musical harmony.",
            course="Music Theory I",
            questions=MIRA_QUESTIONS
        )

if __name__ == "__main__":
    main()
