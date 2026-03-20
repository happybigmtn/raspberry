import json

with open('/tmp/botcoin-challenge.json', 'r') as f:
    data = json.load(f)

companies = data['companies']

metrics = {c: {'founded': None, 'public': None, 'de': None, 'sat': None, 'q3_growth': None, 'q4_rev': None} for c in companies}

# From manual inspection of explicit metrics:
# Founded & Public:
metrics['Zeno Core'].update({'founded': 2003, 'public': True})
metrics['Sola Global'].update({'founded': 1999, 'public': False})
metrics['Quil Sciences'].update({'founded': 1980, 'public': True})
metrics['Giga Industries'].update({'founded': 2018, 'public': False})
metrics['Viro Group'].update({'founded': 2004, 'public': False})
metrics['Juno Analytics'].update({'founded': 1979, 'public': True})
metrics['Vex Micro'].update({'founded': 1999, 'public': False})
metrics['Reva Nexus'].update({'founded': 2000, 'public': True})
metrics['Halo Sphere'].update({'founded': 1972, 'public': True})
metrics['Echo Forge'].update({'founded': 1988, 'public': True})
metrics['Nava Net'].update({'founded': 1997, 'public': True})
metrics['Nova Hub'].update({'founded': 1997, 'public': False})
metrics['Mira Materials'].update({'founded': 2006, 'public': False})
metrics['Coda Net'].update({'founded': 1984, 'public': False})
metrics['Opti Industries'].update({'founded': 1974, 'public': False})
metrics['Tala Micro'].update({'founded': 1970, 'public': False})
metrics['Vex Data'].update({'founded': 1994, 'public': True})
metrics['Halo Sciences'].update({'founded': 1980, 'public': False})
metrics['Nava Tech'].update({'founded': 2014, 'public': False})
metrics['Flux Energy'].update({'founded': 2007, 'public': True})

# DE and SAT explicitly:
metrics['Coda Net'].update({'de': 0.4, 'sat': 1.4})
metrics['Reva Nexus'].update({'de': 0.1, 'sat': 2.9})
metrics['Nava Net'].update({'de': 4.7, 'sat': 1.4})
metrics['Flux Data'] = {'de': 2.7, 'sat': 6.3} # Flux Data not in companies list? Wait, let's check. Flux Energy is!
# Oh wait, Flux Data is in text but maybe not companies? "Flux Energy" is in companies.
metrics['Sola Global'].update({'de': 1.0, 'sat': 7.4})
metrics['Opti Industries'].update({'de': 1.2, 'sat': 2.3})

# Relative DE and SAT
metrics['Juno Analytics'].update({'de': 4.7 - 0.5, 'sat': 1.4 + 4.1}) # relative to Nava Net (4.7, 1.4). DE = 4.2, SAT = 5.5
metrics['Tala Micro'].update({'de': 4.2 - 0.1, 'sat': 5.5 + 4.1}) # relative to Juno Analytics. DE = 4.1, SAT = 9.6
metrics['Quil Sciences'].update({'de': 0.4 + 0.9, 'sat': 1.4 + 8.3}) # relative to Coda Net. DE = 1.3, SAT = 9.7
metrics['Halo Sciences'].update({'de': 0.4 + 3.6, 'sat': 1.4 + 6.4}) # relative to Coda Net. DE = 4.0, SAT = 7.8
metrics['Mira Materials'].update({'de': 0.4 + 2.4, 'sat': 1.4 + 6.3}) # relative to Coda Net. DE = 2.8, SAT = 7.7
metrics['Nova Hub'].update({'de': 0.4 + 1.8, 'sat': 1.4 + 4.6}) # relative to Coda Net. DE = 2.2, SAT = 6.0
metrics['Vex Data'].update({'de': 2.2 + 2.6, 'sat': 6.0 - 3.9}) # relative to Nova Hub. DE = 4.8, SAT = 2.1
metrics['Zeno Core'].update({'de': 4.8 - 1.3, 'sat': 2.1 + 3.1}) # relative to Vex Data. DE = 3.5, SAT = 5.2
metrics['Viro Group'].update({'de': 3.5 - 0.3, 'sat': 5.2 + 4.3}) # relative to Zeno Core. DE = 3.2, SAT = 9.5
metrics['Giga Industries'].update({'de': 4.8 - 3.0, 'sat': 2.1 + 4.5}) # relative to Vex Data. DE = 1.8, SAT = 6.6
metrics['Vex Micro'].update({'de': 4.8 - 2.7, 'sat': 2.1 + 2.3}) # relative to Vex Data. DE = 2.1, SAT = 4.4
metrics['Nava Tech'].update({'de': 2.1 + 0.8, 'sat': 4.4 - 2.3}) # relative to Vex Micro. DE = 2.9, SAT = 2.1
metrics['Flux Energy'].update({'de': 0.1 + 1.1, 'sat': 2.9 + 1.9}) # relative to Reva Nexus. DE = 1.2, SAT = 4.8
metrics['Echo Forge'].update({'de': 1.2 + 2.1, 'sat': 4.8 + 1.3}) # relative to Flux Energy. DE = 3.3, SAT = 6.1
metrics['Halo Sphere'].update({'de': 4.7 - 2.9, 'sat': 1.4 + 0.2}) # relative to Nava Net. DE = 1.8, SAT = 1.6

# Growths
growths = {}
# Explicit growths:
growths['Sola Global'] = -20
growths['Reva Nexus'] = 41
growths['Nava Net'] = 8
growths['Coda Net'] = -6
growths['Opti Industries'] = 43

# Relative growths (Q3 / late-year)
# [paragraph_4] Mira Materials and Sola Global: 6 points favoring Mira Materials = -20 + 6 = -14
growths['Mira Materials'] = -14
# [paragraph_77] Nova Hub growth surpassed Mira Materials by 30 points = -14 + 30 = 16
growths['Nova Hub'] = 16
# [paragraph_15] Nova Hub outpaced Giga Industries by 3 points => Giga Industries = 16 - 3 = 13
growths['Giga Industries'] = 13
# [paragraph_97] Quil Sciences trailing Giga Industries by 30 points = 13 - 30 = -17
growths['Quil Sciences'] = -17
# [paragraph_68] Zeno Core late-year momentum outstripped Nova Hub's by 19 points = 16 + 19 = 35
growths['Zeno Core'] = 35
# [paragraph_59] Viro Group's mid-year... wait. Q3 is paragraph 84: Viro Group outgrew Echo Forge in Q3 by 3 points. Need Echo Forge.
# [paragraph_69] Halo Sphere Q3 momentum outstripped Nova Hub by 5 points = 16 + 5 = 21
growths['Halo Sphere'] = 21
# [paragraph_19] Flux Energy third-quarter growth differential with Halo Sphere was 16 points, favoring Flux Energy = 21 + 16 = 37
growths['Flux Energy'] = 37
# [paragraph_35] Echo Forge and Halo Sciences: 41 points favoring Echo Forge. Wait, need Halo Sciences.
# [paragraph_88] Reva Nexus outpaced Halo Sciences in Q3 growth by 54 points => Halo Sciences = 41 - 54 = -13
growths['Halo Sciences'] = -13
growths['Echo Forge'] = -13 + 41 # = 28
growths['Viro Group'] = 28 + 3 # = 31

# [paragraph_5] Vex Data posted late-year growth 19 points below Nava Net's = 8 - 19 = -11
growths['Vex Data'] = -11
# [paragraph_98] Juno Analytics posted Q3 growth 19 points below Vex Data's = -11 - 19 = -30
growths['Juno Analytics'] = -30
# [paragraph_46] Tala Micro posted growth 32 points stronger than Coda Net = -6 + 32 = 26
growths['Tala Micro'] = 26
# [paragraph_44] Nava Tech saw 21 points more late-year growth than Vex Data = -11 + 21 = 10
growths['Nava Tech'] = 10
# [paragraph_20] Vex Micro outgrew Halo Sciences in Q3 by 36 points = -13 + 36 = 23
growths['Vex Micro'] = 23

# Update metrics with growths
for c in companies:
    if c in growths:
        metrics[c]['q3_growth'] = growths[c]

# Revenues Q4:
revs = {}
revs['Sola Global'] = 3962
revs['Reva Nexus'] = 2769
revs['Nava Net'] = 1868
revs['Coda Net'] = 4612
revs['Opti Industries'] = 2997
# [paragraph_4] Mira Materials earning 340 more than Sola Global = 3962 + 340 = 4302
revs['Mira Materials'] = 4302
# [paragraph_77] Nova Hub trailing Mira Materials by 2831 = 4302 - 2831 = 1471
revs['Nova Hub'] = 1471
# [paragraph_15] Giga Industries ran 521 above Nova Hub = 1471 + 521 = 1992
revs['Giga Industries'] = 1992
# [paragraph_97] Quil Sciences earning 1636 more than Giga Industries = 1992 + 1636 = 3628
revs['Quil Sciences'] = 3628
# [paragraph_68] Zeno Core Q4 top line 1152 below Nova Hub = 1471 - 1152 = 319
revs['Zeno Core'] = 319
# [paragraph_69] Halo Sphere fell 526 short of Nova Hub = 1471 - 526 = 945
revs['Halo Sphere'] = 945
# [paragraph_19] Flux Energy reported 541 ahead of Halo Sphere = 945 + 541 = 1486
revs['Flux Energy'] = 1486
# [paragraph_88] Halo Sciences reported 1099 ahead of Reva Nexus = 2769 + 1099 = 3868
revs['Halo Sciences'] = 3868
# [paragraph_35] Halo Sciences outearned Echo Forge by 1725 = 3868 - 1725 = 2143
revs['Echo Forge'] = 2143
# [paragraph_84] Viro Group posted 811 more than Echo Forge = 2143 + 811 = 2954
revs['Viro Group'] = 2954

# [paragraph_5] Vex Data earning 285 more than Nava Net = 1868 + 285 = 2153
revs['Vex Data'] = 2153
# [paragraph_98] Juno Analytics surpassed Vex Data by 2778 = 2153 + 2778 = 4931
revs['Juno Analytics'] = 4931
# [paragraph_46] Tala Micro results 4098 lower than Coda Net = 4612 - 4098 = 514
revs['Tala Micro'] = 514
# [paragraph_44] Nava Tech came in 1103 behind Vex Data = 2153 - 1103 = 1050
revs['Nava Tech'] = 1050
# [paragraph_20] Halo Sciences outearned Vex Micro by 2527 = 3868 - 2527 = 1341
revs['Vex Micro'] = 1341

for c in companies:
    if c in revs:
        metrics[c]['q4_rev'] = revs[c]

# Question 2: Second-earliest founding year
q2_sorted = sorted(companies, key=lambda c: metrics[c]['founded'] if metrics[c]['founded'] else 9999)
print(f"Q2: {q2_sorted[1]} ({metrics[q2_sorted[1]]['founded']})")

# Question 6: Decline in Q3 (growth < 0), highest D/E ratio
q6_cands = [c for c in companies if metrics[c]['q3_growth'] is not None and metrics[c]['q3_growth'] < 0]
q6_ans = max(q6_cands, key=lambda c: metrics[c]['de'])
print(f"Q6: {q6_ans} (DE={metrics[q6_ans]['de']}, Q3={metrics[q6_ans]['q3_growth']})")

# Question 8: DE < 2.4, SAT > 7.3, largest Q4 rev
q8_cands = [c for c in companies if metrics[c]['de'] is not None and metrics[c]['de'] < 2.4 and metrics[c]['sat'] is not None and metrics[c]['sat'] > 7.3]
q8_ans = max(q8_cands, key=lambda c: metrics[c]['q4_rev'])
print(f"Q8: {q8_ans} (DE={metrics[q8_ans]['de']}, SAT={metrics[q8_ans]['sat']}, Q4={metrics[q8_ans]['q4_rev']})")

# Question 10: Publicly traded, smallest D/E
q10_cands = [c for c in companies if metrics[c]['public'] == True]
q10_ans = min(q10_cands, key=lambda c: metrics[c]['de'])
print(f"Q10: {q10_ans} (DE={metrics[q10_ans]['de']})")

