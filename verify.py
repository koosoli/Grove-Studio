"""Independent integration checks. Outputs stay in this app's shots directory."""
from pathlib import Path
import hashlib
import json
import math
import os
import struct
from io import BytesIO
from PIL import Image
from urllib.parse import unquote
import subprocess

ROOT = Path(__file__).resolve().parent
EXE = Path(os.environ['GROVE_VERIFY_EXE']) if 'GROVE_VERIFY_EXE' in os.environ else next((p for p in [ROOT/'target/release/grove-studio.exe', ROOT/'target/debug/grove-studio.exe'] if p.exists()), None)
assert EXE, 'Build the Rust executable first'
OUT = Path(os.environ.get('GROVE_VERIFY_OUT', ROOT/'shots/verify'))
OUT.mkdir(parents=True, exist_ok=True)

def run(*args, ok=True):
    p = subprocess.run([str(EXE), *map(str,args)], cwd=ROOT, capture_output=True, text=True, timeout=45)
    assert (p.returncode == 0) == ok, (args,p.stdout,p.stderr)
    return json.loads(p.stdout) if ok else p

def gltf(path):
    faces_checked=[0,0]
    doc = json.loads(path.read_text())
    data = (path.parent/unquote(doc['buffers'][0]['uri'])).read_bytes()
    assert doc['asset']['version'] == '2.0'
    assert len(data) == doc['buffers'][0]['byteLength']
    assert len(doc['materials']) == len(doc['meshes'][0]['primitives']) == 2
    assert doc['materials'][1]['doubleSided']
    assert len(doc['images']) == 3
    assert doc['materials'][1].get('alphaMode','OPAQUE') == 'OPAQUE'
    for image in doc['images']:
        view=doc['bufferViews'][image['bufferView']]
        png=data[view['byteOffset']:view['byteOffset']+view['byteLength']]
        im=Image.open(BytesIO(png))
        assert im.format=='PNG' and im.size==(512,512)
    relief=doc['materials'][0]['extras']['parallax']
    assert relief['requiresEngineShader'] and relief['layers']==16
    assert relief['scale']==doc['extras']['settings']['parallax_depth']
    assert doc['textures'][relief['heightTexture']['index']]['source']==len(doc['images'])-2
    assert doc['textures'][doc['materials'][0]['normalTexture']['index']]['source']==len(doc['images'])-1
    height_view=doc['bufferViews'][doc['images'][-2]['bufferView']]
    height_image=Image.open(BytesIO(data[height_view['byteOffset']:height_view['byteOffset']+height_view['byteLength']]))
    assert height_image.getextrema()[1]-height_image.getextrema()[0]>100
    arrays=[]
    for a in doc['accessors']:
        v=doc['bufferViews'][a['bufferView']]
        assert v['byteOffset'] % 4 == 0
        assert v['byteOffset'] + v['byteLength'] <= len(data)
        dims={'VEC2':2,'VEC3':3,'VEC4':4,'SCALAR':1}[a['type']]
        fmt='f' if a['componentType']==5126 else 'I'
        count=a['count']*dims
        assert count*4 == v['byteLength']
        values=struct.unpack_from('<'+fmt*count,data,v['byteOffset'])
        assert all(math.isfinite(x) for x in values)
        arrays.append([values[i:i+dims] for i in range(0,len(values),dims)])
    triangles=0
    heights=[]
    for primitive in doc['meshes'][0]['primitives']:
        att=primitive['attributes']
        positions=arrays[att['POSITION']]
        normals=arrays[att['NORMAL']]
        indices=[v[0] for v in arrays[primitive['indices']]]
        assert max(indices)<len(positions)
        assert len(indices)%3==0
        assert len(positions)==len(normals)==len(arrays[att['TEXCOORD_0']])
        assert all(abs(sum(f*f for f in n)-1)<1e-4 for n in normals)
        tangents=arrays[att['TANGENT']]
        assert len(tangents)==len(positions)
        assert all(abs(sum(f*f for f in t[:3])-1)<1e-4 and t[3] in (-1,1) for t in tangents)
        assert all(abs(sum(n[i]*t[i] for i in range(3)))<1e-4 for n,t in zip(normals,tangents))
        assert all(0<=f<=1 for rgba in arrays[att['COLOR_0']] for f in rgba)
        wind=arrays[att['_WIND']]
        assert len(wind)==len(positions)
        assert all(0<=w[0]<=1 and w[1]==primitive['material'] for w in wind)
        assert all(abs(w[0]-(max(0,min(1,p[1]/doc['extras']['settings']['height'])))**2)<1e-5 for p,w in zip(positions,wind))
        for tri in zip(indices[::3],indices[1::3],indices[2::3]):
            a,b,c=[positions[i] for i in tri]
            ab=[b[i]-a[i] for i in range(3)]
            ac=[c[i]-a[i] for i in range(3)]
            cross=[ab[1]*ac[2]-ab[2]*ac[1],ab[2]*ac[0]-ab[0]*ac[2],ab[0]*ac[1]-ab[1]*ac[0]]
            faces_checked[0]+=1
            if sum(v*v for v in cross)<=1e-16: faces_checked[1]+=1
        heights.extend(p[1] for p in positions)
        triangles+=len(indices)//3
    assert faces_checked[1] == 0, faces_checked
    return triangles, max(heights)-min(heights)

results=[]
for species in ['solstice-oak','solstice-birch','solstice-fir','solstice-bush','solstice-poseidon-oak']:
    out=OUT/f'{species}.gltf'
    result=run('--headless','--preset',species,'--out',out,'--lod','all')
    counts=[]
    for f in result['files']:
        p=Path(f['path'])
        count,h=gltf(p)
        assert count==f['stats']['triangles']
        assert abs(h-f['stats']['height_m']) < 1e-3
        counts.append(count)
    assert counts[2]<counts[1]<counts[0]<900000
    assert counts[2]<counts[0]/2
    before=hashlib.sha256((OUT/f'{species}_lod0.bin').read_bytes()).hexdigest()
    run('--headless','--preset',species,'--out',out,'--lod','all')
    after=hashlib.sha256((OUT/f'{species}_lod0.bin').read_bytes()).hexdigest()
    assert before==after
    results.append({'step':f'{species} geometry, deterministic export, height and LOD budgets','passed':True,'triangles':counts})

obj=run('--headless','--preset','solstice-oak','--out',OUT/'oak.obj','--format','obj','--lod','0')
lines=(OUT/'oak.obj').read_text().splitlines()
vertices=sum(s.startswith('v ') for s in lines)
normals=sum(s.startswith('vn ') for s in lines)
uvs=sum(s.startswith('vt ') for s in lines)
faces=[s.split()[1:] for s in lines if s.startswith('f ')]
assert vertices==normals==uvs==obj['files'][0]['stats']['vertices']
assert len(faces)==obj['files'][0]['stats']['triangles']
assert all(1<=int(i)<=vertices for f in faces for v in f for i in v.split('/'))
assert (OUT/'oak.mtl').exists()
assert (OUT/'oak.bark.png').exists()
assert 'map_d' not in (OUT/'oak.mtl').read_text()
assert (OUT/'oak.bark-height.png').exists() and (OUT/'oak.bark-normal.png').exists()
assert 'disp oak.bark-height.png' in (OUT/'oak.mtl').read_text()
batch=run('--headless','--preset','solstice-fir','--out',OUT/'batch.gltf','--lod','2','--count','3')
assert len(batch['files'])==3
assert len({hashlib.sha256(Path(f['path']).with_suffix('.bin').read_bytes()).digest() for f in batch['files']})==3
for args in [('--height','NaN'),('--height','99'),('--preset','banana'),('--preset','oak'),('--forest-preset','grove'),('--count','0'),('--lod','4'),('--format','fbx'),('--seed',),('--bogus',),('--season','snow'),('--parallax-depth','NaN'),('--parallax-depth','0.5')]:
    run('--headless',*args,ok=False)
results.append({'step':'OBJ topology, distinct seeded batch and invalid input rejection','passed':True})

for species in ['solstice-oak','solstice-birch','solstice-fir']:
    winter=run('--headless','--preset',species,'--season','winter','--out',OUT/f'{species}-winter.gltf','--lod','all')
    summer=run('--headless','--preset',species,'--inspect')
    for f, warm in zip(winter['files'],summer['lods']):
        count,h=gltf(Path(f['path']))
        assert count==f['stats']['triangles']
        cold_doc=json.loads(Path(f['path']).read_text())
        warm_doc=json.loads((OUT/f"{species}_lod{f['stats']['lod']}.gltf").read_text())
        def leaf_count(d):return d['accessors'][d['meshes'][0]['primitives'][1]['attributes']['POSITION']]['count']
        if species!='solstice-fir':assert 0<leaf_count(cold_doc)<leaf_count(warm_doc)/20
        else:assert leaf_count(cold_doc)==leaf_count(warm_doc)
results.append({'step':'Winter exports at every LOD: sparse deciduous foliage and evergreen pine','passed':True})

native = run('--headless','--preset','solstice-oak','--out',OUT/'poseidon/oak.p3d','--lod','all')['files'][0]
native_path=Path(native['path'])
mlod=native_path.read_bytes()
assert mlod[:8]==b'MLOD\x01\x01\x00\x00'
assert struct.unpack_from('<I',mlod,8)[0]==4
assert [entry['resolution'] for entry in native['lods']]==[1,2,4]
assert [entry['source']['triangles'] for entry in native['lods']]==results[0]['triangles']
assert mlod.count(b'#Property#\0')==8 and mlod.count(b'map'+bytes(61)+b'tree'+bytes(60))==4
assert b'component01\0' in mlod and b'#Mass#\0' in mlod
assert all(Path(path).is_file() for path in native['sidecars'])
assert native_path.with_suffix('.leaves_ca.paa').read_bytes().startswith(bytes.fromhex('05ff')+b'GGATGALF')
assert 'TreeAdvTrunk' in native_path.with_suffix('.bark.rvmat').read_text()
results.append({'step':'Poseidon model, three native visual LODs, collision, materials and sway metadata','passed':True})

for args in [('--output-root',), ('--output-root',''), ('--bogus',),
             ('--headless',), ('--output-root','--headless'),
             ('--output-root','first','--output-root','second')]:
    p=subprocess.run([str(EXE),'--mcp',*args],input='',text=True,capture_output=True,cwd=ROOT,timeout=10)
    assert p.returncode!=0 and not p.stdout and p.stderr.strip(), (args,p.stdout,p.stderr)
results.append({'step':'MCP startup rejects missing, unknown, conflicting and duplicate options','passed':True})

requests=[
    {'jsonrpc':'2.0','id':1,'method':'initialize','params':{'protocolVersion':'2025-03-26','capabilities':{},'clientInfo':{'name':'verify','version':'1'}}},
    {'jsonrpc':'2.0','method':'notifications/initialized'},
    {'jsonrpc':'2.0','id':2,'method':'tools/list'},
    {'jsonrpc':'2.0','id':3,'method':'tools/call','params':{'name':'tree_inspect','arguments':{'settings':{'species':'solstice-birch','seed':75}}}},
    {'jsonrpc':'2.0','id':4,'method':'tools/call','params':{'name':'tree_generate','arguments':{'output':'mcp/oak.gltf','lod':'all'}}},
    {'jsonrpc':'2.0','id':5,'method':'tools/call','params':{'name':'tree_generate','arguments':{'output':'../escape.gltf'}}},
    {'jsonrpc':'2.0','id':6,'method':'tools/call','params':{'name':'tree_inspect','arguments':{'settings':{'height':99}}}},
    {'jsonrpc':'2.0','id':7,'method':'ping'},
    {'jsonrpc':'2.0','id':8,'method':'tools/call','params':{'name':'tree_generate','arguments':{'output':'mcp/winter.gltf','lod':'2','settings':{'species':'solstice-birch','season':3,'parallax_depth':0.06}}}},
    {'jsonrpc':'2.0','id':9,'method':'tools/call','params':{'name':'tree_inspect','arguments':{'settings':{'season':4}}}},
    {'jsonrpc':'2.0','id':10,'method':'tools/call','params':{'name':'tree_inspect','arguments':{'settings':{'parallax_depth':0.09}}}},
]
p=subprocess.run([str(EXE),'--mcp','--output-root',str(OUT/'agent')],input='\n'.join(map(json.dumps,requests))+'\n',text=True,capture_output=True,cwd=ROOT,timeout=45)
assert p.returncode==0,(p.stdout,p.stderr)
responses=[json.loads(s) for s in p.stdout.splitlines()]
assert [r['id'] for r in responses]==list(range(1,11))
assert responses[0]['result']['protocolVersion']=='2025-03-26'
assert {tool['name'] for tool in responses[1]['result']['tools']}=={'tree_presets','tree_inspect','tree_generate','tree_preview'}
assert not responses[2]['result']['isError']
assert not responses[3]['result']['isError']
assert responses[4]['result']['isError'] and responses[5]['result']['isError']
export=json.loads(responses[3]['result']['content'][0]['text'])
for f in export['files']:gltf(Path(f['path']))
assert not responses[7]['result']['isError']
winter_export=json.loads(responses[7]['result']['content'][0]['text'])
for f in winter_export['files']:
    gltf(Path(f['path']))
    assert f['settings']['season']==3 and abs(f['settings']['parallax_depth']-0.06)<1e-6
assert responses[8]['result']['isError'] and responses[9]['result']['isError']
settings_schema=responses[1]['result']['tools'][1]['inputSchema']['properties']['settings']['properties']
assert settings_schema['season']['maximum']==3 and settings_schema['parallax_depth']['maximum']==0.08
results.append({'step':'MCP initialize, tools, inspect, mesh generation, notifications, errors and ping','passed':True})
report={'version':1,'steps':results}
(OUT/'report.json').write_text(json.dumps(report,indent=2))
print(json.dumps(report,indent=2))
