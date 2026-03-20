import json
import re

with open('/tmp/botcoin-challenge.json', 'r') as f:
    data = json.load(f)

paragraphs = data['doc'].split('\n\n')

# First, extract explicit metrics
growths = {c: None for c in data['companies']}
revs = {c: [None, None, None, None] for c in data['companies']}
emps = {c: None for c in data['companies']}
founded = {c: None for c in data['companies']}

# Explicit growths
growths['Myra Arc'] = 30
growths['Flux Data'] = 47
growths['Myra Global'] = -22
growths['Opti Materials'] = -23
growths['Tala Prime'] = 10

# Relative Q4 growths
# [paragraph_5] Axio Bio saw 9 points more fourth-quarter growth than Myra Arc did = 30 + 9 = 39
growths['Axio Bio'] = 39

# [paragraph_14] Tala Prime grew 9 points faster than Prim Edge in closing = Prim Edge = 10 - 9 = 1
growths['Prim Edge'] = 1

# [paragraph_15] Tala Pulse grew 5 points faster than Tala Global in Q4 => Tala Pulse = Tala Global + 5

# [paragraph_28] Dyna Forge outgrew Opti Materials in Q4 by 35 points = -23 + 35 = 12
growths['Dyna Forge'] = 12

# [paragraph_36] Nova Synth outgrew Opti Materials in fourth-quarter by 8 points = -23 + 8 = -15
growths['Nova Synth'] = -15

# [paragraph_42] closing-quarter growth at Arlo Dynamics tracked Myra Global's pace closely, differing by 1 point... hmm, 'differing by 1' usually means we need more context. Let's assume +/- 1. Let's see if we have other hints. If Arlo Dynamics trailed/surpassed? "tracked closely, differing by only 1" - usually we might need the exact sign, but maybe it's not the highest anyway.

# [paragraph_47] Q4 growth at Sero Works ran 69 points above Opti Materials's rate = -23 + 69 = 46
growths['Sero Works'] = 46

# [paragraph_48] Orba Tech's closing-quarter growth trailed Myra Global's by 5 points = -22 - 5 = -27
growths['Orba Tech'] = -27

# [paragraph_51] Prim Dynamics's growth rate surpassed Orba Tech's by 17 points = -27 + 17 = -10
growths['Prim Dynamics'] = -10

# [paragraph_53] Fuse Flow saw 75 points more closing-quarter growth than Onyx Hub did => Fuse Flow = Onyx Hub + 75
# [paragraph_57] Prim Dynamics outpaced Onyx Hub in closing-quarter growth by 19 points => Onyx Hub = Prim Dynamics - 19 = -10 - 19 = -29
growths['Onyx Hub'] = -29
growths['Fuse Flow'] = -29 + 75 # = 46

# [paragraph_56] Nero Synth grew 36 points faster than Tala Pulse in closing-quarter => Nero Synth = Tala Pulse + 36
# [paragraph_88] Nero Synth's fourth-quarter momentum outstripped Prim Dynamics's by 40 points => Nero Synth = -10 + 40 = 30
growths['Nero Synth'] = 30
# Therefore Tala Pulse = 30 - 36 = -6
growths['Tala Pulse'] = -6
# And Tala Global = -6 - 5 = -11
growths['Tala Global'] = -11

# [paragraph_59] Nova Synth outpaced Myra Sys in fourth-quarter growth by 6 points => Myra Sys = -15 - 6 = -21
growths['Myra Sys'] = -21

# [paragraph_68] The closing-quarter growth differential between Rune Flow and Myra Sys was 29 points, favoring Rune Flow => Rune Flow = -21 + 29 = 8
growths['Rune Flow'] = 8

for c, g in growths.items():
    print(f"{c} Q4 Growth: {g}")

