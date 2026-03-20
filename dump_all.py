import json
import re

with open('/tmp/botcoin-challenge.json', 'r') as f:
    data = json.load(f)

doc = data['doc']
companies = data['companies']
paragraphs = doc.split('\n\n')

# 1. Gather all explicit metric paragraphs
# "Across the fiscal year X recorded..."
# "X employs Y staff, carries a debt-to-equity ratio of Z, and holds a satisfaction score of W"
# "X traces its origins to ... went public ... privately held"

for p in paragraphs:
    print(p)
