# Data Folder

## Purpose
This folder stores raw source snapshots and generated JSON for a Palworld base-planner data pipeline.

## Raw Sources
Raw HTML files are in `data/raw` and were downloaded from:
- `https://paldeck.cc` (pals, items, buildings, skills, technology, breeding pages)
- `https://paldb.cc/en` (work-suitability and production/workload reference pages)
- `https://docs.palworldgame.com` (official configuration reference page)

## How Data Was Obtained
- Download method: direct HTTP GET from PowerShell (`Invoke-WebRequest`) in this workspace.
- Saved as snapshot files prefixed with `__` and moved into `data/raw`.
- These files are unprocessed raw captures intended as source-of-truth inputs for later JSON extraction.

## Notes
- Some pages are dynamic and embed structured data inside HTML/script payloads.
- Next step is to parse these raw files into normalized planner JSON datasets.
