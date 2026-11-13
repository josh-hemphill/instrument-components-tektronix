"""Run the manual-authored oracle helper in isolation; do not overwrite active data."""
import shutil,subprocess,sys,tempfile
from pathlib import Path
root=Path(__file__).resolve().parent
with tempfile.TemporaryDirectory(prefix='tds1002-oracle-')as temporary:
 out=Path(temporary);helper='build-tds1002-oracle.py';shutil.copy2(root/helper,out/helper)
 subprocess.run([sys.executable,str(out/helper)],cwd=out,check=True)
 for name in ['tds1002.catalog-cases.json','tds1002-fixture.json']:assert(root/name).read_bytes()==(out/name).read_bytes(),name
 print('PASS all334 independent TDS1002 cases and fixture reproduce byte-for-byte')
