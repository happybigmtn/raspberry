import json
import re

with open('/tmp/botcoin-challenge.json', 'r') as f:
    data = json.load(f)

doc = data['doc']
paragraphs = doc.split('\n\n')

def get_p(n):
    for p in paragraphs:
        if p.startswith(f'[paragraph_{n}]'):
            return p
    return ""

def print_p(text):
    print("---")
    for p in paragraphs:
        if text.lower() in p.lower():
            print(p)

companies = data['companies']

for c in companies:
    print(f"Company: {c}")
    print_p(c)
    print("====================")
