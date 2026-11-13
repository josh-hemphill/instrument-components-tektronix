"""Join the independent TDS1002 oracle; never reads a production backend."""
import argparse,json
from pathlib import Path
root=Path(__file__).resolve().parents[2]
parser=argparse.ArgumentParser();parser.add_argument('--check',action='store_true');args=parser.parse_args()
p=root/'cases.json';document=json.loads(p.read_text());baseline=[c for c in document['cases'] if not c['id'].startswith('tektronix.tds1002|')]
assert len(baseline)==118
new=json.loads((root/'spec/acceptance/tds1002.catalog-cases.json').read_text())['cases'];assert len(new)==334
combined=baseline+new;assert len(combined)==len({c['id']for c in combined})==452
document['cases']=combined
if args.check:assert document==json.loads(p.read_text()),'Acceptance shard drift'
else:p.write_text(json.dumps(document,indent=2)+'\n')
print('PASS 118 preserved Tek cases +334 independent TDS1002 cases=452')
