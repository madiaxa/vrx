from pathlib import Path
import subprocess,tempfile,struct
root=Path(__file__).resolve().parent.parent
icons=root/'assets/icons'; source=root/'assets/logo.png'
def resize(dest,size,fmt='png'):
 subprocess.run(['sips','-s','format',fmt,'-z',str(size),str(size),str(source),'--out',str(dest)],check=True,stdout=subprocess.DEVNULL)
resize(icons/'vrx-256.png',256)
with tempfile.TemporaryDirectory() as folder:
 folder=Path(folder);chunks=b''
 for tag,size in [('icp4',16),('icp5',32),('icp6',64),('ic07',128),('ic08',256),('ic09',512),('ic10',1024)]:
  p=folder/f'{size}.png';resize(p,size);data=p.read_bytes();chunks+=tag.encode()+struct.pack('>I',len(data)+8)+data
 (icons/'VRX.icns').write_bytes(b'icns'+struct.pack('>I',len(chunks)+8)+chunks)
 entries=[]
 for size in (16,24,32,48,64,128,256):
  p=folder/f'ico{size}.png';resize(p,size);entries.append((size,p.read_bytes()))
 offset=6+16*len(entries);head=struct.pack('<HHH',0,1,len(entries));payload=b''
 for size,data in entries:
  head+=struct.pack('<BBBBHHII',size%256,size%256,0,0,1,32,len(data),offset);payload+=data;offset+=len(data)
 (icons/'VRX.ico').write_bytes(head+payload)
 for size,label in [(64,'window')]:
  bmp=folder/f'{label}.bmp';resize(bmp,size,'bmp');data=bmp.read_bytes();offset=struct.unpack_from('<I',data,10)[0];w,h=struct.unpack_from('<ii',data,18);bits=struct.unpack_from('<H',data,28)[0]
  if w!=size or abs(h)!=size or bits not in (24,32):raise ValueError('Unsupported icon BMP')
  stride=((w*bits+31)//32)*4;raw=bytearray()
  for y in range(size):
   for x in range(size):
    start=offset+(size-1-y if h>0 else y)*stride+x*(bits//8);b,g,r=data[start:start+3];raw.extend((r,g,b,255))
  (icons/f'{label}.rgba').write_bytes(raw)
print('Logo preserved; native icon variants generated')
