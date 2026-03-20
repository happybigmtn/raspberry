import json
import re

with open('/tmp/botcoin-challenge.json', 'r') as f:
    data = json.load(f)

paragraphs = data['doc'].split('\n\n')

companies = data['companies']

metrics = {c: {'founded': None, 'public': None, 'de': None, 'sat': None, 'q3_growth': None, 'q4_rev': None} for c in companies}

# Explicit metrics
for p in paragraphs:
    for c in companies:
        if c in p or c.replace(' ', '') in p:
            if 'traces its origins to' in p or 'was founded in' in p:
                m1 = re.search(r'(?:traces its origins to|was founded in) (\d{4})', p)
                if m1: metrics[c]['founded'] = int(m1.group(1))
                if 'went public' in p: metrics[c]['public'] = True
                if 'remains privately held' in p: metrics[c]['public'] = False
            
            # Explicit D/E and Satisfaction
            if 'carries a debt-to-equity ratio of' in p and 'holds a satisfaction score of' in p:
                m2 = re.search(r'debt-to-equity ratio of ([\d\.]+)', p)
                if m2: metrics[c]['de'] = float(m2.group(1))
                m3 = re.search(r'satisfaction score of ([\d\.]+)', p)
                if m3: metrics[c]['sat'] = float(m3.group(1))

# Explicit growths and revenues
growths = {c: None for c in companies}
revs = {c: [None, None, None, None] for c in companies}

# Look for explicit revenues
for p in paragraphs:
    if 'Across the fiscal year' in p or 'Revenue for' in p or 'posted' in p:
        # Hand-parse explicit:
        pass

# Reva Nexus explicit
revs['Reva Nexus'] = [3749, 3132, 3233, 2769]
# Sola Global explicit
revs['Sola Global'] = [486, 4378, 4555, 3962]
# Nava Net explicit
revs['Nava Net'] = [2069, 2833, 2065, 1868]
# Coda Net explicit
revs['Coda Net'] = [3734, 4052, 2951, 4612]
# Opti Industries explicit
revs['Opti Industries'] = [4596, 1981, 2563, 2997]

# Let's write a python parser for the relative equations like I did before.
# It's faster if I just parse everything.
print("Finished setting up initial script")
