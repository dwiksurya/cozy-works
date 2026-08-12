#!/usr/bin/env python3
"""Poll GitHub Actions status for dwiksurya/cozy-works, print summary."""
import json, os, sys, time, urllib.request

TOKEN = os.environ.get("GITHUB_TOKEN", "")
REPO = "dwiksurya/cozy-works"

def api(path):
    req = urllib.request.Request(f"https://api.github.com{path}", headers={
        "Authorization": f"token {TOKEN}",
        "Accept": "application/vnd.github+json",
        "User-Agent": "cozy-works-watchdog",
    })
    with urllib.request.urlopen(req, timeout=15) as r:
        return json.load(r)

def main():
    runs = api(f"/repos/{REPO}/actions/runs?per_page=5")
    out = []
    for r in runs.get("workflow_runs", []):
        status = r["status"]
        concl = r.get("conclusion") or "—"
        msg = (r.get("head_commit") or {}).get("message", "")[:45].replace("\n", " ")
        out.append(f"{r['id']} {status:12s} {concl:10s} {msg}")
    print("\n".join(out) if out else "no runs")

if __name__ == "__main__":
    main()
