#!/usr/bin/python3

import sys
query = sys.argv[1:]

for line in sys.stdin:
    terms = line.strip().split("|")

    found = 0
    for q in query:
        for term in terms:
            if term == q:
            #if term.startswith(q):
                found += 1
                break
    if found >= len(query):
        print(terms)
