import React from 'react';
import {
  AbsoluteFill, Audio, Img, Sequence, interpolate, spring, staticFile,
  useCurrentFrame, useVideoConfig,
} from 'remotion';

const C={bg:'#07100f',surface:'#101d1b',surface2:'#152522',line:'#28413d',line2:'#3f6a63',cyan:'#64f6e9',cyan2:'#22cfc2',text:'#edf6f3',muted:'#9cb2ad',dim:'#617874'};
const clamp={extrapolateLeft:'clamp',extrapolateRight:'clamp'} as const;
const p=(f:number,a:number,b:number)=>interpolate(f,[a,b],[0,1],clamp);
const ease=(v:number)=>1-Math.pow(1-v,4);
const fade=(f:number,d:number,tail=12)=>Math.min(p(f,0,12),interpolate(f,[d-tail,d],[1,0],clamp));
const rnd=(i:number,s:number)=>{const x=Math.sin(i*12.9898+s*78.233)*43758.5453;return x-Math.floor(x)};

const Backdrop:React.FC=()=>{
  const f=useCurrentFrame();
  return <AbsoluteFill style={{background:C.bg,backgroundImage:`radial-gradient(circle at ${70+Math.sin(f/90)*2}% 24%,rgba(34,207,194,.12),transparent 35%),linear-gradient(rgba(100,246,233,.025) 1px,transparent 1px),linear-gradient(90deg,rgba(100,246,233,.025) 1px,transparent 1px)`,backgroundSize:'100% 100%,72px 72px,72px 72px'}}/>;
};

const Logo:React.FC<{size:number;glow?:number}>=({size,glow=0})=><div style={{width:size,height:size,borderRadius:size*.13,overflow:'hidden',background:'#f7fbfa',boxShadow:`0 0 ${50+glow*70}px rgba(100,246,233,${.1+glow*.22})`}}><Img src={staticFile('locastra-icon.png')} style={{display:'block',width:'100%',height:'100%',objectFit:'cover'}}/></div>;
const Kicker:React.FC<{children:React.ReactNode;vertical:boolean}>=({children,vertical})=><div style={{font:`700 ${vertical?33:27}px/1 Cascadia Code,Consolas,monospace`,letterSpacing:vertical?5:4,color:C.cyan}}>{children}</div>;
const Title:React.FC<{children:React.ReactNode;vertical:boolean}>=({children,vertical})=><div style={{font:`760 ${vertical?76:70}px/1.08 "Segoe UI Variable","Microsoft YaHei",sans-serif`,letterSpacing:-2,color:C.text}}>{children}</div>;
const Chip:React.FC<{children:React.ReactNode;accent?:boolean;vertical?:boolean}>=({children,accent,vertical})=><div style={{border:`1px solid ${accent?C.cyan2:C.line}`,background:accent?'rgba(34,207,194,.13)':'rgba(16,29,27,.78)',borderRadius:999,padding:vertical?'14px 22px':'11px 18px',color:accent?C.cyan:C.muted,font:`650 ${vertical?27:23}px/1 "Segoe UI Variable","Microsoft YaHei",sans-serif`,whiteSpace:'nowrap'}}>{children}</div>;
const Header:React.FC<{index:string;title:React.ReactNode;vertical:boolean;side?:React.ReactNode}>=({index,title,vertical,side})=><div style={{width:'100%',maxWidth:vertical?930:1640,display:'flex',flexDirection:vertical?'column':'row',justifyContent:'space-between',alignItems:vertical?'flex-start':'flex-end',gap:30}}><div><Kicker vertical={vertical}>{index}</Kicker><div style={{marginTop:16}}><Title vertical={vertical}>{title}</Title></div></div>{side}</div>;

const Opening:React.FC<{vertical:boolean;duration:number}>=({vertical,duration})=>{
  const f=useCurrentFrame(), mark=spring({frame:f-3,fps:30,config:{damping:18,stiffness:100}}), name=p(f,26,62), ring=ease(p(f,0,58));
  return <AbsoluteFill style={{opacity:fade(f,duration),alignItems:'center',justifyContent:'center'}}>
    <svg width={vertical?930:1220} height={vertical?930:760} viewBox="0 0 1220 760" style={{position:'absolute'}}>
      {[0,1,2].map(i=><circle key={i} cx="610" cy="348" r={150+i*82} fill="none" stroke={i===0?C.cyan2:C.line} strokeWidth={i===0?2:1} strokeDasharray={`${ring*(220+i*100)} 1800`} opacity={.5-i*.1} transform={`rotate(${-85+i*37} 610 348)`}/>)}
      {Array.from({length:26},(_,i)=>{const a=i/26*Math.PI*2+f*.0025,r=264+(i%3)*26;return <circle key={i} cx={610+Math.cos(a)*r} cy={348+Math.sin(a)*r} r={i%5===0?4:2.2} fill={i%5===0?C.cyan:C.line2} opacity={ring*(.35+(i%4)*.13)}/>})}
    </svg>
    <div style={{position:'relative',display:'flex',flexDirection:'column',alignItems:'center',transform:`translateY(${(1-mark)*34}px) scale(${.9+mark*.1})`,opacity:mark}}>
      <Logo size={vertical?210:165} glow={ring}/>
      <div style={{marginTop:38,overflow:'hidden',padding:'0 20px'}}><div style={{font:`800 ${vertical?105:108}px/1 "Segoe UI Variable",sans-serif`,letterSpacing:8+(1-name)*22,color:C.text,opacity:name,transform:`translateY(${(1-name)*36}px)`,textShadow:'0 0 40px rgba(100,246,233,.12)'}}>LOCASTRA</div></div>
      <div style={{width:vertical?650:760,height:2,background:C.line,margin:'30px 0 25px',overflow:'hidden'}}><div style={{height:'100%',width:`${name*100}%`,background:C.cyan,boxShadow:'0 0 20px rgba(100,246,233,.65)'}}/></div>
      <Kicker vertical={vertical}>让本地 AI 真正属于你</Kicker>
    </div>
  </AbsoluteFill>;
};

const SourceMerge:React.FC<{vertical:boolean;duration:number}>=({vertical,duration})=>{
  const f=useCurrentFrame(),draw=p(f,10,54),conv=ease(p(f,56,111));
  const sources=[['ModelScope','MS'],['hf-mirror','镜'],['Hugging Face','HF'],['本地 GGUF','GG']];
  const w=vertical?930:1550,h=vertical?860:570,x0=vertical?140:170,x1=vertical?735:1270,ys=vertical?[165,340,515,690]:[105,225,345,465],mid=vertical?430:285;
  return <AbsoluteFill style={{opacity:fade(f,duration),padding:vertical?'130px 70px':'72px 100px',alignItems:'center'}}>
    <Header index="01 · MODEL SOURCES" title={<>多个模型源<br/>汇入一个入口</>} vertical={vertical} side={<div style={{display:'flex',gap:10}}><Chip accent>国内优先</Chip><Chip>全站搜索</Chip></div>}/>
    <div style={{position:'relative',width:w,height:h,marginTop:vertical?120:50}}>
      <svg width={w} height={h} style={{position:'absolute',inset:0,overflow:'visible'}}>{ys.map((y,i)=><path key={i} d={vertical?`M ${x0},${y} C 370,${y} 520,430 ${x1},430`:`M ${x0},${y} C 520,${y} 760,285 ${x1},285`} fill="none" stroke={i===0?C.cyan2:C.line2} strokeWidth={i===0?2.2:1.4} pathLength={1} strokeDasharray="1" strokeDashoffset={1-draw} opacity={.35+i*.1}/>)}</svg>
      {sources.map((s,i)=>{const y=ys[i],x=x0+(x1-x0)*conv,cy=y+(mid-y)*Math.pow(conv,1.65),shrink=conv<.72?1-conv*.35:.75*(1-(conv-.72)/.28),inn=spring({frame:f-i*6,fps:30,config:{damping:18,stiffness:120}});return <div key={s[0]} style={{position:'absolute',left:x,top:cy,transform:`translate(-50%,-50%) scale(${Math.max(.01,shrink*inn)})`,opacity:inn*(conv>.97?0:1),display:'flex',alignItems:'center',gap:14}}><div style={{width:vertical?80:68,height:vertical?80:68,borderRadius:18,display:'grid',placeItems:'center',background:C.surface2,border:`1px solid ${i===0?C.cyan2:C.line2}`,color:i===0?C.cyan:C.text,font:`800 ${vertical?25:21}px Cascadia Code,monospace`}}>{s[1]}</div><div style={{font:`650 ${vertical?27:24}px "Segoe UI Variable",sans-serif`,color:C.muted,whiteSpace:'nowrap'}}>{s[0]}</div></div>})}
      <div style={{position:'absolute',left:x1,top:mid,transform:'translate(-50%,-50%)'}}><div style={{width:vertical?185:160,height:vertical?185:160,borderRadius:'50%',border:`1px solid ${C.cyan2}`,background:'radial-gradient(circle,rgba(100,246,233,.18),rgba(16,29,27,.94) 66%)',display:'grid',placeItems:'center',boxShadow:`0 0 ${40+conv*55}px rgba(100,246,233,.2)`,transform:`scale(${.82+conv*.18})`}}><div style={{textAlign:'center'}}><div style={{font:`800 ${vertical?38:34}px Cascadia Code,monospace`,color:C.cyan}}>GGUF</div><div style={{font:`650 ${vertical?22:18}px "Segoe UI Variable",sans-serif`,color:C.muted,marginTop:8}}>兼容候选</div></div></div>{[0,1,2].map(i=><div key={i} style={{position:'absolute',inset:-26-i*26,borderRadius:'50%',border:`1px solid rgba(100,246,233,${.22-i*.05})`,transform:`scale(${1+Math.sin((f+i*11)/14)*.025})`}}/>)}</div>
    </div>
  </AbsoluteFill>;
};

const Hardware:React.FC<{vertical:boolean;duration:number}>=({vertical,duration})=>{
  const f=useCurrentFrame(),inn=spring({frame:f,fps:30,config:{damping:20,stiffness:95}}),scan=p(f,20,86),specs=[['GPU','RTX 5090','32 GB'],['CPU','9950X3D','16 C'],['RAM','SYSTEM','32 GB']],ring=vertical?610:510;
  return <AbsoluteFill style={{opacity:fade(f,duration),padding:vertical?'125px 65px':'70px 100px',alignItems:'center'}}>
    <Header index="02 · HARDWARE FIT" title={<>不用猜参数<br/>先判断能不能跑</>} vertical={vertical} side={<div style={{display:'flex',gap:10}}><Chip>示例配置</Chip><Chip accent>流畅</Chip><Chip>可运行</Chip><Chip>不建议</Chip></div>}/>
    <div style={{marginTop:vertical?125:55,width:vertical?930:1500,display:'flex',flexDirection:vertical?'column':'row',alignItems:'center',justifyContent:'center',gap:vertical?80:130}}>
      <div style={{position:'relative',width:ring,height:ring,transform:`scale(${.9+inn*.1})`,opacity:inn}}>{[0,1,2].map(i=><div key={i} style={{position:'absolute',inset:i*48,borderRadius:'50%',border:`${i===1?2:1}px solid ${i===1?C.cyan2:C.line}`,transform:`rotate(${f*(i===1?.22:-.12)+i*25}deg)`,borderLeftColor:i===1?'transparent':undefined,borderBottomColor:i===0?'transparent':undefined}}/>)}<div style={{position:'absolute',inset:ring*.28,borderRadius:32,background:C.surface,border:`1px solid ${C.line2}`,display:'grid',placeItems:'center',boxShadow:'0 30px 90px rgba(0,0,0,.38)'}}><div style={{textAlign:'center'}}><div style={{font:`800 ${vertical?52:42}px Cascadia Code,monospace`,color:C.text}}>Q4_K_M</div><div style={{font:`650 ${vertical?24:20}px "Segoe UI Variable",sans-serif`,color:C.cyan,marginTop:12}}>推荐量化</div></div></div><div style={{position:'absolute',left:'50%',top:'50%',width:ring*.49,height:2,background:`linear-gradient(90deg,transparent,${C.cyan})`,transformOrigin:'0 50%',transform:`rotate(${scan*340-120}deg)`,boxShadow:'0 0 14px rgba(100,246,233,.45)'}}/></div>
      <div style={{width:vertical?850:660,display:'grid',gap:18}}>{specs.map((s,i)=>{const q=spring({frame:f-14-i*9,fps:30,config:{damping:18,stiffness:110}});return <div key={s[0]} style={{height:vertical?120:102,borderRadius:20,border:`1px solid ${i===0?C.cyan2:C.line}`,background:i===0?'rgba(34,207,194,.10)':C.surface,display:'grid',gridTemplateColumns:'110px 1fr auto',alignItems:'center',padding:'0 28px',opacity:q,transform:`translateX(${(1-q)*55}px)`}}><div style={{font:`700 ${vertical?24:20}px Cascadia Code,monospace`,color:i===0?C.cyan:C.dim}}>{s[0]}</div><div style={{font:`700 ${vertical?29:25}px "Segoe UI Variable",sans-serif`,color:C.text}}>{s[1]}</div><div style={{font:`700 ${vertical?28:24}px Cascadia Code,monospace`,color:i===0?C.cyan:C.muted}}>{s[2]}</div></div>})}<div style={{display:'flex',gap:12,marginTop:8,opacity:p(f,62,82)}}><Chip accent vertical={vertical}>CUDA</Chip><Chip vertical={vertical}>Vulkan</Chip><Chip vertical={vertical}>CPU 混合卸载</Chip></div></div>
    </div>
  </AbsoluteFill>;
};

const Download:React.FC<{vertical:boolean;duration:number}>=({vertical,duration})=>{
  const f=useCurrentFrame(),lines=[['ModelScope',C.cyan],['hf-mirror.com',C.text],['Hugging Face',C.muted]],target=vertical?760:1370,ys=vertical?[240,430,620]:[125,245,365],w=vertical?880:1500,progress=Math.min(38,Math.round(p(f,18,94)*38));
  return <AbsoluteFill style={{opacity:fade(f,duration),padding:vertical?'130px 70px':'72px 100px',alignItems:'center'}}>
    <Header index="03 · RESUMABLE DOWNLOAD" title={<>国内友好下载<br/>断了也能接着下</>} vertical={vertical} side={<div style={{display:'flex',gap:10}}><Chip accent>Range 续传</Chip><Chip>.part 校验</Chip></div>}/>
    <div style={{position:'relative',width:w,height:vertical?900:570,marginTop:vertical?110:45,borderRadius:30,border:`1px solid ${C.line}`,background:'linear-gradient(145deg,rgba(16,29,27,.9),rgba(9,18,16,.72))',overflow:'hidden'}}>
      {lines.map((l,i)=>{const route=p(f,8+i*10,52+i*8),packet=((f-35-i*13)%68+68)%68/68,start=vertical?300:340;return <React.Fragment key={l[0]}><div style={{position:'absolute',left:vertical?55:75,top:ys[i]-36,font:`700 ${vertical?25:22}px Cascadia Code,monospace`,color:l[1]}}>{l[0]}</div><div style={{position:'absolute',left:start,top:ys[i],height:2,width:(target-start)*route,background:i===0?C.cyan2:C.line2}}/><div style={{position:'absolute',left:start+(target-start)*packet,top:ys[i]-7,width:14,height:14,borderRadius:4,background:i===0?C.cyan:C.text,boxShadow:i===0?'0 0 20px rgba(100,246,233,.7)':undefined,opacity:route}}/></React.Fragment>})}
      <div style={{position:'absolute',left:target,top:vertical?430:285,transform:'translate(-50%,-50%)',width:vertical?230:210,height:vertical?230:210,borderRadius:34,border:`1px solid ${C.cyan2}`,background:C.surface2,display:'grid',placeItems:'center',boxShadow:'0 0 50px rgba(100,246,233,.12)'}}><div style={{textAlign:'center'}}><div style={{font:`800 ${vertical?40:36}px Cascadia Code,monospace`,color:C.cyan}}>.GGUF</div><div style={{font:`650 ${vertical?22:19}px "Segoe UI Variable",sans-serif`,color:C.muted,marginTop:10}}>完整校验</div></div></div>
      <div style={{position:'absolute',left:vertical?55:75,right:vertical?55:75,bottom:vertical?105:70}}><div style={{display:'flex',justifyContent:'space-between',alignItems:'end'}}><div><div style={{font:`700 ${vertical?29:25}px "Segoe UI Variable",sans-serif`,color:C.text}}>自动切换来源 · 中断恢复</div><div style={{font:`600 ${vertical?22:18}px Cascadia Code,monospace`,color:C.muted,marginTop:8}}>DeepSeek-R1-Distill-Qwen-7B.Q4_K_M.gguf</div></div><div style={{font:`800 ${vertical?46:40}px Cascadia Code,monospace`,color:C.cyan}}>{progress}%</div></div><div style={{height:vertical?13:10,borderRadius:99,background:'#22312e',marginTop:20,overflow:'hidden'}}><div style={{height:'100%',width:`${progress/38*100}%`,background:`linear-gradient(90deg,${C.cyan2},${C.cyan})`,boxShadow:'0 0 22px rgba(100,246,233,.65)'}}/></div><div style={{display:'flex',justifyContent:'space-between',marginTop:14,font:`650 ${vertical?21:17}px Cascadia Code,monospace`,color:C.dim}}><span>13.0 MB/s</span><span>HTTP RANGE · RETRY · SHA256</span></div></div>
    </div>
  </AbsoluteFill>;
};

const Inference:React.FC<{vertical:boolean;duration:number}>=({vertical,duration})=>{
  const f=useCurrentFrame(),N=160,gather=p(f,10,72),out=p(f,55,duration-30),stages=['PROMPT','INFERENCE','TOKEN STREAM'],w=vertical?920:1530,h=vertical?1040:610;
  return <AbsoluteFill style={{opacity:fade(f,duration),padding:vertical?'120px 65px':'68px 100px',alignItems:'center'}}>
    <Header index="04 · LOCAL INFERENCE" title={<>模型留在电脑里<br/>内容不上传</>} vertical={vertical} side={<div style={{display:'flex',gap:10}}><Chip accent>127.0.0.1</Chip><Chip>离线可用</Chip><Chip>流式输出</Chip></div>}/>
    <div style={{position:'relative',width:w,height:h,marginTop:vertical?95:40}}>
      <svg width={w} height={h} style={{position:'absolute',inset:0}}>{Array.from({length:N},(_,i)=>{const sx=rnd(i,1)*w,sy=vertical?100+rnd(i,2)*720:70+rnd(i,2)*470,tx=vertical?w*.5:w*.34,ty=vertical?420:300,k=Math.min(1,Math.max(0,(gather-rnd(i,4)*.22)/.78)),x=sx+(tx-sx)*ease(k),y=sy+(ty-sy)*ease(k);return <circle key={i} cx={x} cy={y} r={i%9===0?4:2.2} fill={i%7===0?C.cyan:C.line2} opacity={.25+k*.62}/>})}</svg>
      <div style={{position:'absolute',left:vertical?'50%':'34%',top:vertical?420:300,transform:'translate(-50%,-50%)'}}><div style={{width:vertical?310:280,height:vertical?310:280,transform:'rotate(45deg)',borderRadius:42,border:`1px solid ${C.cyan2}`,background:'linear-gradient(145deg,rgba(100,246,233,.18),rgba(16,29,27,.98))',boxShadow:'0 0 80px rgba(100,246,233,.16)',display:'grid',placeItems:'center'}}><div style={{transform:'rotate(-45deg)',textAlign:'center'}}><div style={{font:`800 ${vertical?50:44}px Cascadia Code,monospace`,color:C.text}}>GGUF</div><div style={{font:`650 ${vertical?23:20}px "Segoe UI Variable",sans-serif`,color:C.cyan,marginTop:10}}>LOCAL ENGINE</div></div></div></div>
      <div style={{position:'absolute',left:vertical?40:w*.55,right:vertical?40:0,top:vertical?585:40,bottom:vertical?0:25}}>
        <svg width="100%" height="100%" viewBox="0 0 700 470" preserveAspectRatio="none" style={{position:'absolute',inset:0}}>
          {stages.slice(0,2).map((_,i)=>{const q=p(f,58+i*24,78+i*24);return <path key={i} d={`M ${118+i*225} 235 C ${205+i*225} ${170+i*45} ${250+i*225} ${300-i*40} ${343+i*225} 235`} fill="none" stroke={i===1?C.cyan2:C.line2} strokeWidth={2.2} pathLength={1} strokeDasharray="1" strokeDashoffset={1-q} opacity={.8}/>})}
          {Array.from({length:34},(_,i)=>{const q=p(f,78+i*.7,98+i*.7),x=75+(i%9)*68,y=365+Math.sin(i*1.7+f*.08)*22;return <circle key={i} cx={x} cy={y} r={i%5===0?5:3} fill={i%4===0?C.cyan:C.line2} opacity={q*(.35+(i%3)*.2)} transform={`translate(${out*45} 0)`}/>})}
        </svg>
        <div style={{position:'absolute',left:0,right:0,top:vertical?40:65,display:'grid',gridTemplateColumns:'repeat(3,1fr)',gap:vertical?20:28}}>{stages.map((text,i)=>{const q=spring({frame:f-48-i*22,fps:30,config:{damping:18,stiffness:105}});return <div key={text} style={{display:'flex',flexDirection:'column',alignItems:'center',gap:14,opacity:q,transform:`translateY(${(1-q)*24}px)`}}><div style={{width:vertical?116:126,height:vertical?116:126,borderRadius:'50%',border:`1px solid ${i===2?C.cyan:C.line2}`,background:`radial-gradient(circle,rgba(100,246,233,${i===2?.2:.08}),rgba(7,16,15,.9))`,boxShadow:i===2?'0 0 45px rgba(100,246,233,.22)':undefined,display:'grid',placeItems:'center'}}><span style={{font:`800 ${vertical?31:33}px Cascadia Code,monospace`,color:i===2?C.cyan:C.text}}>{String(i+1).padStart(2,'0')}</span></div><span style={{font:`700 ${vertical?20:21}px Cascadia Code,monospace`,letterSpacing:1.5,color:i===2?C.cyan:C.muted,whiteSpace:'nowrap'}}>{text}</span></div>})}</div>
        <div style={{position:'absolute',left:vertical?70:60,right:vertical?70:60,bottom:vertical?35:40,height:7,borderRadius:99,background:'#263834',overflow:'hidden'}}><div style={{height:'100%',width:`${out*100}%`,background:`linear-gradient(90deg,${C.cyan2},${C.cyan})`,boxShadow:'0 0 18px rgba(100,246,233,.55)'}}/></div>
      </div>
    </div>
  </AbsoluteFill>;
};

const Outro:React.FC<{vertical:boolean;duration:number}>=({vertical,duration})=>{
  const f=useCurrentFrame(),collapse=ease(p(f,0,60)),lock=spring({frame:f-38,fps:30,config:{damping:18,stiffness:105}}),url=p(f,55,76),features=['模型发现','硬件适配','稳定下载','本地推理'];
  return <AbsoluteFill style={{opacity:fade(f,duration,4),alignItems:'center',justifyContent:'center'}}>
    {features.map((x,i)=>{const a=i/features.length*Math.PI*2-Math.PI/2,r=(vertical?430:500)*(1-collapse);return <div key={x} style={{position:'absolute',left:'50%',top:'44%',transform:`translate(calc(-50% + ${Math.cos(a)*r}px),calc(-50% + ${Math.sin(a)*r}px)) scale(${1-collapse*.35})`,opacity:1-collapse}}><Chip accent={i===0} vertical={vertical}>{x}</Chip></div>})}
    <div style={{display:'flex',flexDirection:'column',alignItems:'center',transform:`scale(${.86+lock*.14})`,opacity:lock}}><Logo size={vertical?210:165} glow={1}/><div style={{font:`800 ${vertical?94:92}px/1 "Segoe UI Variable",sans-serif`,letterSpacing:8,color:C.text,marginTop:30}}>LOCASTRA</div><div style={{font:`650 ${vertical?35:30}px/1.3 "Segoe UI Variable","Microsoft YaHei",sans-serif`,color:C.muted,marginTop:16}}>Windows 本地 AI，现已开源</div><div style={{width:vertical?820:980,height:1,background:C.line,margin:'38px 0 30px'}}><div style={{height:2,width:`${url*100}%`,background:C.cyan,boxShadow:'0 0 18px rgba(100,246,233,.6)'}}/></div><div style={{font:`750 ${vertical?39:40}px/1 Cascadia Code,Consolas,monospace`,color:'#bafff8',opacity:.25+url*.75,transform:`translateY(${(1-url)*15}px)`,whiteSpace:'nowrap',textShadow:'0 0 24px rgba(100,246,233,.42)'}}>github.com/Nobel-BY/Locastra</div><div style={{display:'flex',gap:12,marginTop:30,opacity:p(f,72,90)}}><Chip accent vertical={vertical}>0.9.0</Chip><Chip vertical={vertical}>Windows x64</Chip><Chip vertical={vertical}>免费下载</Chip></div></div>
  </AbsoluteFill>;
};

const SHOTS={opening:{from:0,duration:105},sources:{from:93,duration:150},hardware:{from:231,duration:165},download:{from:384,duration:150},inference:{from:522,duration:228},outro:{from:738,duration:162}};
const SFX=[
  {from:SHOTS.sources.from+2,duration:42,src:'air-zoom-vacuum.mp3',volume:.25},
  {from:SHOTS.hardware.from+5,duration:40,src:'whoosh-fast.mp3',volume:.22},
  {from:SHOTS.download.from+3,duration:42,src:'sweep-metal-quick.mp3',volume:.23},
  {from:SHOTS.inference.from+3,duration:44,src:'air-zoom-vacuum.mp3',volume:.21},
  {from:SHOTS.outro.from+4,duration:55,src:'sweep-metal-quick.mp3',volume:.24},
  {from:SHOTS.outro.from+46,duration:58,src:'impact-deep-whoosh.mp3',volume:.30},
  {from:SHOTS.outro.from+88,duration:60,src:'shimmer-sparkle-sweep.mp3',volume:.18},
];
const Soundtrack:React.FC<{bgm:boolean}>=({bgm})=><>{bgm?<Audio src={staticFile('audio/house-vibez.mp3')} startFrom={7} volume={f=>Math.min(.54,f/48*.54,(900-f)/55*.54)}/>:null}{SFX.map((s,i)=><Sequence key={i} from={s.from} durationInFrames={s.duration}><Audio src={staticFile(`audio/${s.src}`)} volume={s.volume}/></Sequence>)}</>;

export const LocastraPromo:React.FC<{vertical:boolean;bgm:boolean}>=({vertical,bgm})=>{
  const {width,height}=useVideoConfig();
  return <AbsoluteFill style={{width,height,overflow:'hidden',color:C.text}}><Backdrop/>
    <Sequence from={SHOTS.opening.from} durationInFrames={SHOTS.opening.duration}><Opening vertical={vertical} duration={SHOTS.opening.duration}/></Sequence>
    <Sequence from={SHOTS.sources.from} durationInFrames={SHOTS.sources.duration}><SourceMerge vertical={vertical} duration={SHOTS.sources.duration}/></Sequence>
    <Sequence from={SHOTS.hardware.from} durationInFrames={SHOTS.hardware.duration}><Hardware vertical={vertical} duration={SHOTS.hardware.duration}/></Sequence>
    <Sequence from={SHOTS.download.from} durationInFrames={SHOTS.download.duration}><Download vertical={vertical} duration={SHOTS.download.duration}/></Sequence>
    <Sequence from={SHOTS.inference.from} durationInFrames={SHOTS.inference.duration}><Inference vertical={vertical} duration={SHOTS.inference.duration}/></Sequence>
    <Sequence from={SHOTS.outro.from} durationInFrames={SHOTS.outro.duration}><Outro vertical={vertical} duration={SHOTS.outro.duration}/></Sequence>
    <Soundtrack bgm={bgm}/>
  </AbsoluteFill>;
};
