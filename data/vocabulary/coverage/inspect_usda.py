#!/usr/bin/env python3
"""Reproduce USDA description searches and verify reviewed evidence against pinned ZIPs.

Download the four URLs in source-snapshots.json to their local_filename names,
then run with --archives DIR. No extraction, API key or network access is needed.
Optional candidate CSV output belongs outside the repository. Searches discover
candidates; they never assign or change the hand-reviewed match judgments.
"""
import argparse
import collections
import csv
import hashlib
import io
import json
from pathlib import Path
import re
import unicodedata
import zipfile

ROOT = Path(__file__).resolve().parent


def normalized(value):
    return "".join(c for c in unicodedata.normalize("NFKD", value.casefold())
                   if not unicodedata.combining(c))


def require(condition, message):
    if not condition:
        raise ValueError(message)


def query_pattern(reviews, group):
    terms = collections.defaultdict(set)
    for row in reviews:
        for query in row["queries"][group]:
            terms[normalized(query)].add(row["number"])
    # Expand nested phrases too: a longest hit for garlic powder also finds garlic.
    expanded = {
        term: set().union(*(numbers for query, numbers in terms.items()
                           if re.search(r"(?<!\w)" + re.escape(query) + r"(?!\w)", term)))
        for term in terms
    }
    # Lookahead also permits phrases overlapping at different start positions.
    pattern = re.compile(r"(?=(?<!\w)(" + "|".join(
        re.escape(t) for t in sorted(terms, key=lambda t: (-len(t), t))) + r")(?!\w))")
    return pattern, expanded


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archives", type=Path, required=True)
    parser.add_argument("--candidates", type=Path,
                        help="Optional generated CSV outside the checkout")
    args = parser.parse_args()
    reviews = json.loads((ROOT / "usda-review.json").read_text())
    snapshots = json.loads((ROOT / "source-snapshots.json").read_text())["usda"]["archives"]
    patterns = {group: query_pattern(reviews, group) for group in ("generic", "branded")}
    selected = collections.defaultdict(dict)
    for row in reviews:
        for group in ("generic", "branded"):
            for record in row[group + "_records"]:
                key = (record["snapshot"], record["source_id"])
                for field in ("description", "data_type", "publication_date"):
                    require(field not in selected[key] or selected[key][field] == record[field],
                            f"Conflicting review evidence: {key} {field}")
                    selected[key][field] = record[field]
                if group == "branded":
                    for field in ("brand_owner", "brand_name", "ingredients",
                                  "market_country", "discontinued_date"):
                        selected[key][field] = record[field]
    require(len({s["key"] for s in snapshots}) == 4, "Expected four distinct USDA snapshots")
    found, metadata_found = set(), set()
    counts, hits = {}, {}
    candidate_handle = None
    try:
        writer = None
        if args.candidates:
            path = args.candidates.resolve()
            checkout = ROOT.parents[2]
            require(not path.is_relative_to(checkout), "Keep generated candidates outside the checkout")
            candidate_handle = path.open("w", newline="")
            writer = csv.writer(candidate_handle, lineterminator="\n")
            writer.writerow(["snapshot", "fdc_id", "data_type", "description", "benchmark_numbers"])
        for snapshot in snapshots:
            path = args.archives / snapshot["local_filename"]
            require(path.stat().st_size == snapshot["bytes"], f"Wrong archive length: {path}")
            with path.open("rb") as handle:
                digest = hashlib.file_digest(handle, "sha256").hexdigest()
            require(digest == snapshot["sha256"], f"Wrong archive SHA-256: {path}")
            kind = snapshot["key"]
            group = "branded" if kind == "branded" else "generic"
            pattern, terms = patterns[group]
            row_counts, hit_counts = collections.Counter(), collections.Counter()
            with zipfile.ZipFile(path) as archive:
                for member in snapshot["members"]:
                    info = archive.getinfo(member["name"])
                    require(info.file_size == member["bytes"] and
                            f"{info.CRC:08x}" == member["crc32"], "ZIP member identity changed")
                member = next(m["name"] for m in snapshot["members"] if m["name"].endswith("/food.csv"))
                with archive.open(member) as raw:
                    for food in csv.DictReader(io.TextIOWrapper(raw, encoding="utf-8-sig")):
                        row_counts[food["data_type"]] += 1
                        if food["data_type"] != snapshot["reviewed_data_type"]:
                            continue
                        numbers = set()
                        for match in pattern.finditer(normalized(food["description"])):
                            numbers.update(terms[match.group(1)])
                        hit_counts.update(numbers)
                        if writer and numbers:
                            writer.writerow([kind, food["fdc_id"], food["data_type"],
                                             food["description"], " ".join(map(str, sorted(numbers)))])
                        key = (kind, food["fdc_id"])
                        if key in selected:
                            require(key not in found, f"Duplicate selected FDC ID: {key}")
                            for field in ("description", "data_type", "publication_date"):
                                require(food[field] == selected[key][field], f"Food evidence mismatch: {key} {field}")
                            found.add(key)
                require(row_counts[snapshot["reviewed_data_type"]] == snapshot["eligible_food_rows"],
                        f"Unexpected eligible food count: {kind}")
                if group == "branded":
                    member = next(m["name"] for m in snapshot["members"] if m["name"].endswith("/branded_food.csv"))
                    with archive.open(member) as raw:
                        for food in csv.DictReader(io.TextIOWrapper(raw, encoding="utf-8-sig")):
                            key = (kind, food["fdc_id"])
                            if key not in selected:
                                continue
                            require(key not in metadata_found, f"Duplicate selected product metadata: {key}")
                            for field in ("brand_owner", "brand_name", "ingredients",
                                          "market_country", "discontinued_date"):
                                require(food[field] == selected[key][field], f"Product evidence mismatch: {key} {field}")
                            metadata_found.add(key)
            counts[kind] = dict(row_counts)
            hits[kind] = dict(sorted(hit_counts.items()))
        require(found == set(selected), "Selected records missing from archives: " + str(set(selected) - found))
        branded_keys = {key for key in selected if key[0] == "branded"}
        require(metadata_found == branded_keys, "Missing selected product ingredient evidence")
        print(json.dumps({"verified_selected_records": len(found),
                          "verified_product_records": len(metadata_found),
                          "food_rows_by_type": counts,
                          "description_candidate_counts": hits}, indent=2))
    finally:
        if candidate_handle:
            candidate_handle.close()


if __name__ == "__main__":
    main()
