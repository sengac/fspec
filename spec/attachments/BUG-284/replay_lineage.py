"""Offline diagnostic using preserved real evidence; never writes source data."""
import copy
import hashlib
import json
from pathlib import Path

ROOT = Path('/tmp/compaction-investigation/private')
BLOBS = Path('/home/rquast/.fspec/blobs')

def digest(value):
    data = value.encode() if isinstance(value, str) else json.dumps(value, sort_keys=True).encode()
    return hashlib.sha256(data).hexdigest()

def restore(value, receipts):
    if isinstance(value, str) and value.startswith('blob:sha256:') and len(value) == 76:
        sha = value[len('blob:sha256:'):]
        src = BLOBS / sha[:2] / sha
        data = src.read_bytes()
        assert hashlib.sha256(data).hexdigest() == sha
        dst = ROOT / 'blobs' / sha
        dst.parent.mkdir(exist_ok=True, mode=0o700)
        dst.write_bytes(data)
        receipts[str(src)] = sha
        return data.decode()
    if isinstance(value, dict):
        if set(value) == {'_blob_ref'}:
            return json.loads(restore(value['_blob_ref'], receipts))
        return {k: restore(v, receipts) for k, v in value.items()}
    if isinstance(value, list):
        return [restore(v, receipts) for v in value]
    return value

def reminder(item):
    if item.get('type') != 'message' or item.get('role') != 'user': return False
    parts = item.get('content', [])
    if isinstance(parts, str): parts = [{'type':'input_text','text':parts}]
    for p in parts:
        text = p.get('text','').strip()
        if p.get('type') != 'input_text' or not text.startswith('<system-reminder>'): return False
        while text.startswith('<system-reminder>'):
            body, sep, rest = text[17:].partition('</system-reminder>')
            if not sep or '<!-- type:compaction-dag -->' in body: return False
            text = rest.strip()
        if text: return False
    return bool(parts)

def assistant(item):
    c = item.get('content', [])
    if item.get('type') == 'message' and item.get('role') == 'assistant' and len(c) == 1:
        if isinstance(c[0],dict) and set(c[0]) == {'type','text'} and c[0]['type'] == 'output_text':
            return c[0]['text']
    return None

def project(items):
    out=[]
    for raw in items:
        if reminder(raw):continue
        it=copy.deepcopy(raw)
        for k in ['id','phase','status']:it.pop(k,None)
        if it['type']=='function_call' and isinstance(it['arguments'],str):it['arguments']=json.loads(it['arguments'])
        if isinstance(it.get('content'),list):
            parts=[]
            for p in it['content']:
                for k in ['annotations','logprobs']:p.pop(k,None)
                if parts and p.get('type')=='output_text' and parts[-1].get('type')=='output_text':parts[-1]['text']+=p['text']
                else:parts.append(p)
            it['content']=parts
        prev=next((p for p in reversed(out) if p['type'] not in ['reasoning','web_search_call']),None)
        if prev is not None and assistant(prev) is not None and assistant(it) is not None:
            prev['content'][0]['text']+=it['content'][0]['text']
        else:out.append(it)
    return out

def incoming(originals):
    items=[]
    for original in originals:
        msg=original['metadata']['message'];role=msg['role']
        for b in msg['content']:
            kind=b['type']
            if kind=='text':items.append({'type':'message','role':role,'content':[{'type':'output_text' if role=='assistant' else 'input_text','text':b['text']}]})
            elif kind=='tool_use':items.append({'type':'function_call','call_id':b.get('call_id',b['id']),'name':b['name'],'arguments':json.dumps(b['input'],ensure_ascii=False,separators=(',',':'))})
            elif kind=='tool_result':
                output=b['content']
                if isinstance(output,list):output='\n'.join(x['text'] for x in output)
                items.append({'type':'function_call_output','call_id':b.get('call_id',b['tool_use_id']),'output':output})
            else:raise ValueError('unsupported '+kind)
    return items

def compare(prior,current):
    queue=copy.deepcopy(current);batch=[]
    for i,it in enumerate(prior):
        call=it['type']=='function_call'
        if not call:batch=[]
        pos=None
        if queue and queue[0]==it:pos=0
        elif call:
            for j,nxt in enumerate(queue):
                if nxt['type']=='function_call':
                    if nxt==it:pos=j
                    else:return i,it,nxt,'tool call ancestry'
                    break
                if nxt['type']!='function_call_output' or nxt['call_id'] not in batch:break
        if pos is not None:
            queue.pop(pos)
            if call:batch.append(it['call_id'])
        elif it['type'] not in ['function_call','reasoning','web_search_call']:
            return i,it,queue[0] if queue else None,'assistant, tool result or DAG ancestry'
    return None

def main():
    receipts={};report=[]
    for p in sorted(ROOT.glob('*/manifest.json')):
        m=json.loads(p.read_text());d={v['id']:v for v in map(json.loads,(p.parent/'originals.jsonl').open())}
        originals=[restore(d[r['message_id']],receipts) for r in m['messages']]
        cp=json.loads((p.parent/(m['provider_state']['state_id']+'.json')).read_text())['checkpoints'][0]['metadata']['codexContinuation']
        wire=incoming(originals);a=project(cp['items']);b=project(wire);first=compare(a,b)
        (p.parent/'reconstructed-input.json').write_text(json.dumps({'model':cp['model'],'input':wire}))
        (p.parent/'checkpoint.json').write_text(json.dumps(cp))
        record={'session_id':p.parent.name,'prior_projected':len(a),'current_projected':len(b),'mismatch':None}
        if first:
            i,expected,actual,error=first
            record['mismatch']={'index':i,'error':error,'expected_type':expected['type'],'actual_type':actual['type'] if actual else None,'expected_hash':digest(expected),'actual_hash':digest(actual)}
            (p.parent/'first-divergence.json').write_text(json.dumps({'expected':expected,'actual':actual},indent=2))
            if expected.get('type')=='function_call_output':record['mismatch']['call_id']=expected['call_id']
            if assistant(expected) is not None and actual and assistant(actual) is not None:
                x,y=assistant(expected),assistant(actual);prefix=next((i for i,(a,b) in enumerate(zip(x,y)) if a!=b),min(len(x),len(y)))
                record['mismatch'].update(expected_text_len=len(x),actual_text_len=len(y),common_prefix=prefix)
        report.append(record)
    (ROOT/'blob-manifest.json').write_text(json.dumps(receipts,indent=2)+'\n')
    Path('/tmp/compaction-investigation/offline-comparison.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report,indent=2))

if __name__=='__main__':main()
