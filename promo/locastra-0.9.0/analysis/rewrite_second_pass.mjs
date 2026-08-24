import fs from 'node:fs';

const path = new URL('../src/LocastraPromo.tsx', import.meta.url);
let text = fs.readFileSync(path, 'utf8');
const replacements = [
  ["top:vertical?62:38,left:vertical?54:64", "top:vertical?130:38,left:vertical?62:64"],
  ["font:`800 ${vertical?26:22}px", "font:`800 ${vertical?32:28}px"],
  ["<BrandBug vertical={vertical} invert={light}/>{children}", "<BrandBug vertical={vertical} invert={light}/><div style={{position:'absolute',inset:vertical?'0 110px 90px 0':'0'}}>{children}</div>"],
  ["${vertical?27:24}px/1 Cascadia", "${vertical?42:32}px/1 Cascadia"],
  ["${vertical?24:20}px/1 \"Segoe UI Variable\"", "${vertical?32:28}px/1 \"Segoe UI Variable\""],
  ["四个来源<br/><span style={{color:C.cyan}}>一次搜全</span>", "多路入口<br/><span style={{color:C.cyan}}>一处管理</span>"],
  ["01 · 全站模型搜索", "01 · 多源模型入口"],
  ["font:`750 ${vertical?27:24}px", "font:`750 ${vertical?34:32}px"],
  ["font:`650 ${vertical?28:25}px", "font:`650 ${vertical?36:32}px"],
  ["size={vertical?96:122}", "size={vertical?106:122}"],
  ["size={vertical?94:118}", "size={vertical?106:118}"],
  ["size={vertical?91:112}", "size={vertical?106:112}"],
  ["size={vertical?100:122}", "size={vertical?108:122}"],
  ["size={vertical?101:128}", "size={vertical?108:128}"],
  ["size={vertical?98:124}", "size={vertical?108:124}"],
  ["font:`700 ${vertical?27:24}px", "font:`700 ${vertical?34:32}px"],
  ["font:'800 18px Cascadia Code'", "font:'800 28px Cascadia Code'"],
  ["font:`800 ${vertical?29:27}px", "font:`800 ${vertical?36:32}px"],
  ["font:`650 ${vertical?21:19}px", "font:`650 ${vertical?30:28}px"],
  ["font:`700 ${vertical?22:19}px", "font:`700 ${vertical?30:28}px"],
  ["font:`800 ${vertical?28:25}px", "font:`800 ${vertical?36:32}px"],
  ["font:`800 ${vertical?27:23}px", "font:`800 ${vertical?34:30}px"],
  ["font:`800 ${vertical?25:22}px", "font:`800 ${vertical?32:30}px"],
  ["font:`800 ${vertical?24:21}px", "font:`800 ${vertical?32:30}px"],
  ["font:`900 ${vertical?31:34}px", "font:`900 ${vertical?38:34}px"],
  ["font:`800 ${vertical?29:26}px", "font:`800 ${vertical?36:30}px"],
  ["font:`800 ${vertical?39:36}px", "font:`800 ${vertical?52:36}px"],
  ["font:`850 ${vertical?36:37}px", "font:`850 ${vertical?58:37}px"],
  ["map(f,0,52,0,100)", "map(f,0,30,0,100)"],
  ["map(f,0,52,8,86)", "map(f,0,30,8,86)"],
  ["const f=useCurrentFrame(),n=Math.round(map(f,0,52,1,32));", "const f=useCurrentFrame();"],
  ["{i===0?`${n} LAYERS`:'READY'}", "{i===0?'AUTO':'READY'}"],
  ["['流式输出','32.0 token/s'],['长上下文','128K']", "['流式输出','STREAMING'],['长上下文','LONG CONTEXT']"],
  ["const q=spring({frame:f-i*5,fps:FPS,config:{damping:15,stiffness:160}}),pulse=1+Math.sin((f-i*4)/5)*.012;", "const q=spring({frame:f-i*5,fps:FPS,config:{damping:15,stiffness:160}}),pulse=f<24?1+Math.sin((f-i*4)/5)*.012:1;"],
  ["map(f,5+i*4,42+i*4,5,92)", "map(f,5+i*4,28+i*4,5,92)"],
  ["RTX 5090", "GPU SCAN"],
  ["流畅运行</div>", "适配结果示例</div>"],
  ["vertical?'0 110px 90px 0':'0'", "vertical?'95px 110px 0 0':'0'"],
  ["hf-mirror → ModelScope", "自动切换可用来源"],
  ["{Math.round(map(f,0,30,8,86))}.4 MB/s", "断点续传"],
  ["url=out(map(f,20,42))", "url=out(map(f,4,20))"],
  ["opacity:map(f,40,52)", "opacity:map(f,18,30)"],
];

for (const [from, to] of replacements) {
  if (!text.includes(from)) console.warn('missing replacement:', from);
  text = text.replaceAll(from, to);
}

const chatStart = text.indexOf('const Chat:');
const chatEnd = text.indexOf('const Triple:');
if (chatStart < 0 || chatEnd < 0 || chatEnd <= chatStart) throw new Error('Chat block not found');
const chat = `const Chat:React.FC<{vertical:boolean}>=({vertical})=>{const f=useCurrentFrame(),q=spring({frame:f,fps:FPS,config:{damping:14,stiffness:145}}),flow=map(f,0,28);return <Scene vertical={vertical} light><div style={{position:'absolute',inset:vertical?'148px 54px 52px':'105px 88px 52px'}}><Kicker vertical={vertical} color={C.teal}>06 · 离线对话</Kicker><div style={{display:'grid',gridTemplateColumns:vertical?'1fr':'1fr 1fr',gap:vertical?45:90,alignItems:'center',height:'88%'}}><div><Big vertical={vertical} size={vertical?108:118}>网络断开<br/><span style={{color:C.teal}}>仍可对话</span></Big><div style={{display:'flex',gap:12,marginTop:30}}><Pill vertical={vertical} light>NETWORK OFF</Pill><Pill vertical={vertical} light accent>LOCAL READY</Pill></div></div><div style={{position:'relative',height:vertical?520:430,display:'grid',placeItems:'center'}}><div style={{position:'absolute',width:vertical?440:400,height:vertical?440:400,borderRadius:'50%',border:'2px solid #b9cec8',transform:\`scale(\${.82+q*.18})\`}}/><div style={{width:vertical?220:200,height:vertical?220:200,borderRadius:48,background:C.dark,display:'grid',placeItems:'center',boxShadow:'0 35px 90px #173a3440'}}><div style={{textAlign:'center'}}><div style={{font:\`900 \${vertical?52:48}px Cascadia Code\`,color:C.cyan}}>GGUF</div><div style={{font:\`800 \${vertical?32:30}px \"Segoe UI Variable\"\`,color:C.ink,marginTop:12}}>本地生成</div></div></div>{Array.from({length:18},(_,i)=>{const a=i/18*Math.PI*2,r=(vertical?260:240)*flow;return <i key={i} style={{position:'absolute',left:'50%',top:'50%',width:i%4===0?14:8,height:i%4===0?14:8,borderRadius:'50%',background:i%4===0?C.teal:'#78968f',transform:\`translate(calc(-50% + \${Math.cos(a)*r}px),calc(-50% + \${Math.sin(a)*r}px))\`,opacity:flow}}/>})}</div></div></div></Scene>};

`;
text = text.slice(0, chatStart) + chat + text.slice(chatEnd);
fs.writeFileSync(path, text, 'utf8');
