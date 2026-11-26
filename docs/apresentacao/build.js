const pptxgen = require("pptxgenjs");
const SK = "/Users/edgar/Library/Application Support/Claude/local-agent-mode-sessions/skills-plugin/bc2f3e38-fafb-415a-99d3-a212e79afa5b/5104a142-5416-480e-b1b7-2895c7e99fbc/skills/pptx";
const { applyTheme } = require(SK + "/scripts/apply_theme.js");

const THEME = { name: "agentq", headFontFace: "Cambria", bodyFontFace: "Calibri",
  colors: { dk1:"0F1B2D", lt1:"FFFFFF", dk2:"1F3A5F", lt2:"EEF2F7",
    accent1:"1FA89A", accent2:"F5A524", accent3:"E4572E", accent4:"6C8EAD", accent5:"8E7DBE", accent6:"3DDC97",
    hlink:"1FA89A", folHlink:"6C8EAD" } };

const pres = new pptxgen();
pres.layout = "LAYOUT_16x9";
pres.theme = { headFontFace: THEME.headFontFace, bodyFontFace: THEME.bodyFontFace };
pres.title = "agentq: migrar para o mundo pós-quântico";
const C = pres.SchemeColor;

const TITLE = (color) => ({ type:"title", options:{ name:"title", x:0.5, y:0.35, w:9, h:0.9, fontSize:30, bold:true, color, valign:"top", margin:0 } });
pres.defineSlideMaster({ title:"LIGHT", background:{ color:"FFFFFF" },
  objects:[ { placeholder: TITLE(C.text1) } ], slideNumber:{ x:9.2, y:5.2, w:0.5, h:0.3, fontSize:10, color:"6C8EAD" } });
pres.defineSlideMaster({ title:"DARK", background:{ color:"0F1B2D" },
  objects:[ { placeholder: TITLE(C.background1) } ] });

const T = (s, text, o) => s.addText(text, Object.assign({ isTextBox:true, margin:0, fontFace:"Calibri" }, o));
const card = (s, x,y,w,h, fill, name, shadow) => s.addShape(pres.shapes.ROUNDED_RECTANGLE, Object.assign({ x,y,w,h, rectRadius:0.12, fill:{color:fill}, line:{color:fill, width:0}, objectName:name },
  shadow ? { shadow:{ type:"outer", color:"0F1B2D", opacity:0.12, blur:8, offset:2, angle:90 } } : {}));
const num = (s, x,y,d, label, fill, txt) => { s.addShape(pres.shapes.OVAL,{x,y,w:d,h:d,fill:{color:fill},line:{color:fill,width:0},objectName:"badge"}); T(s,label,{x,y,w:d,h:d,align:"center",valign:"middle",fontSize:d*28,bold:true,color:txt,fontFace:"Cambria"}); };

// 1. Title
let s = pres.addSlide({ masterName:"DARK" });
T(s,"TRABALHO DE CONCLUSÃO DE CURSO",{x:0.7,y:0.8,w:8,h:0.3,fontSize:12,bold:true,color:"1FA89A",charSpacing:3});
T(s,"Quem gravar seu tráfego hoje pode lê-lo em 2035",{x:0.7,y:1.3,w:8.2,h:1.7,fontSize:38,bold:true,color:"FFFFFF",fontFace:"Cambria",valign:"top"});
T(s,"agentq: encontrando criptografia vulnerável a computadores quânticos e indicando a migração",{x:0.7,y:3.15,w:7.6,h:0.8,fontSize:18,color:"CADCFC",valign:"top"});
T(s,"Edgar Camacho Seabra Ribeiro · Henrico Birochi · Nicholas Birochi · Vítor Agostino Braghittoni",{x:0.7,y:4.8,w:8.6,h:0.3,fontSize:12,color:"8FA6C0"});
s.addNotes("Abertura (1 min). Gancho: se alguém gravar o seu tráfego HTTPS hoje, o que acontece em 2035? Vamos falar de uma ameaça que ainda não existe na prática, mas que já exige decisão hoje. Ao final: por que é um problema de agora e uma ferramenta que construímos para ajudar a tratá-lo.");

// 2. Problema
s = pres.addSlide({ masterName:"LIGHT" });
s.addText("1 · O problema: a criptografia de chave pública tem prazo",{placeholder:"title"});
const cols = [["RSA","Segurança baseada em fatorar números inteiros enormes."],["Diffie-Hellman","Segurança baseada no logaritmo discreto."],["Curvas elípticas","Mesma ideia do logaritmo discreto, em curvas (ECDH, ECDSA)."]];
cols.forEach((c,i)=>{ const x=0.5+i*3.05; card(s,x,1.5,2.85,1.7,"EEF2F7","card",false);
  T(s,c[0],{x:x+0.25,y:1.65,w:2.4,h:0.4,fontSize:20,bold:true,color:"0F1B2D",fontFace:"Cambria"});
  T(s,c[1],{x:x+0.25,y:2.15,w:2.4,h:0.9,fontSize:15,color:"1F3A5F",valign:"top"}); });
card(s,0.5,3.5,9,1.45,"0F1B2D","callout",false);
T(s,[{text:"Algoritmo de Shor (1994)",options:{bold:true,color:"F5A524",breakLine:true}},{text:"Resolve os dois problemas em tempo polinomial num computador quântico. Aumentar a chave não resolve.",options:{color:"FFFFFF"}}],{x:0.85,y:3.65,w:8.3,h:1.15,fontSize:18,valign:"middle"});
s.addNotes("Seção 1 (3 min). Quase toda a internet depende de criptografia de chave pública: cadeado do HTTPS, assinaturas de software, JWT, SSH, carteiras cripto. Funciona porque certos problemas matemáticos são impraticáveis para um computador comum. Em 1994, Peter Shor mostrou que um computador quântico resolve exatamente esses problemas de forma eficiente. Aumentar a chave NÃO resolve: o custo do ataque cresce de forma polinomial, não exponencial. Nota: Grover afeta a criptografia simétrica, mas só enfraquece (AES-256 continua adequado). O problema grave é a chave pública.");

// 3. Qubits
s = pres.addSlide({ masterName:"LIGHT" });
s.addText("A máquina ainda não existe, mas construí-la está ficando mais barato",{placeholder:"title"});
s.addChart(pres.charts.BAR,[{name:"Qubits físicos (milhões)",labels:["Gidney e Ekerå, 2021 (8 horas)","Gidney, 2025 (menos de 1 semana)"],values:[20,1]}],
 { x:0.5,y:1.45,w:5.6,h:3.6,barDir:"col",chartColors:["1FA89A","F5A524"],varyColors:false,showTitle:true,title:"Qubits físicos para quebrar RSA-2048 (milhões)",titleFontSize:14,titleColor:"0F1B2D",titleFontFace:"+mn-lt",
  showValue:true,dataLabelPosition:"outEnd",dataLabelFontSize:14,dataLabelColor:"0F1B2D",dataLabelFontFace:"+mn-lt",showLegend:false,
  catAxisLabelColor:"1F3A5F",catAxisLabelFontSize:11,catAxisLabelFontFace:"+mn-lt",valAxisLabelColor:"1F3A5F",valAxisLabelFontSize:11,valAxisLabelFontFace:"+mn-lt",
  valGridLine:{color:"D5DDE8",size:0.5},catGridLine:{style:"none"},valAxisMaxVal:24 });
T(s,"20×",{x:6.5,y:1.7,w:3,h:1.0,fontSize:60,bold:true,color:"1FA89A",fontFace:"Cambria"});
T(s,"menos qubits em quatro anos",{x:6.5,y:2.7,w:3,h:0.4,fontSize:16,color:"1F3A5F"});
card(s,6.5,3.5,3,1.4,"FFF3DC","aviso",false);
T(s,"São estimativas de recursos, não ataques demonstrados.",{x:6.7,y:3.6,w:2.6,h:1.2,fontSize:15,bold:true,color:"7A4B00",valign:"middle"});
s.addNotes("A estimativa de qubits físicos caiu de cerca de 20 milhões (Gidney e Ekerå, 2021) para menos de 1 milhão (Gidney, 2025). Deixe claro: são ESTIMATIVAS, não ataques demonstrados. O valor 1 no gráfico representa 'menos de 1 milhão'. Transição: se ainda não existe, por que isso é urgente?");

// 4. HNDL
s = pres.addSlide({ masterName:"DARK" });
s.addText("2 · Já é um problema de hoje: harvest now, decrypt later",{placeholder:"title"});
const steps=[["HOJE","O atacante grava tráfego cifrado","F5A524"],["ANOS DEPOIS","Os dados ficam guardados em disco","6C8EAD"],["DIA Q","Um computador quântico decifra tudo","E4572E"]];
steps.forEach((st,i)=>{ const x=0.5+i*3.2; card(s,x,1.7,2.6,1.9,"1F3A5F","step",false);
  T(s,st[0],{x:x+0.25,y:1.9,w:2.1,h:0.35,fontSize:14,bold:true,color:st[2],charSpacing:2});
  T(s,st[1],{x:x+0.25,y:2.35,w:2.1,h:1.1,fontSize:18,color:"FFFFFF",valign:"top"});
  if(i<2) s.addShape(pres.shapes.RIGHT_ARROW,{x:x+2.7,y:2.45,w:0.4,h:0.35,fill:{color:"8FA6C0"},line:{color:"8FA6C0",width:0},objectName:"seta"}); });
T(s,"O atacante não precisa quebrar nada agora. Só armazenar.",{x:0.5,y:4.1,w:9,h:0.5,fontSize:22,bold:true,color:"FFFFFF",fontFace:"Cambria"});
s.addNotes("Seção 2 (3 min). O ataque não começa no dia em que a máquina existir: começa agora, gravando tráfego. Quem faz isso não precisa quebrar nada hoje, só armazenar. Isso afeta o SIGILO. Para assinaturas digitais o risco é outro (falsificação só vale depois que a máquina existir), mas chaves e certificados de longa duração também precisam ser trocados com antecedência.");

// 5. Mosca
s = pres.addSlide({ masterName:"LIGHT" });
s.addText("A conta de Mosca: se x + y > z, o dano já está contratado",{placeholder:"title"});
const mo=[["x","Por quanto tempo o dado precisa ficar secreto","1FA89A"],["y","Quanto tempo a migração leva","F5A524"],["z","Tempo até um computador quântico relevante","E4572E"]];
mo.forEach((m,i)=>{ const x=0.5+i*3.05; card(s,x,1.5,2.85,1.9,"EEF2F7","card",false); num(s,x+0.25,1.7,0.6,m[0],m[2],"FFFFFF");
  T(s,m[1],{x:x+0.25,y:2.45,w:2.4,h:0.85,fontSize:15,color:"0F1B2D",valign:"top"}); });
card(s,0.5,3.7,9,1.2,"0F1B2D","exemplo",false);
T(s,[{text:"Prontuário médico: ",options:{bold:true,color:"F5A524"}},{text:"30 anos de sigilo + 5 de migração > 15 até o computador quântico",options:{color:"FFFFFF"}}],{x:0.8,y:3.8,w:8.4,h:1.0,fontSize:20,valign:"middle"});
s.addNotes("Desigualdade de Mosca (2018): se x + y > z, o dano já está contratado. Exemplo: prontuário médico com 30 anos de sigilo, migração de 5 anos, computador quântico em 15: a conta fecha no vermelho. Contratos, segredos industriais e dados de governo seguem o mesmo raciocínio. Migrar leva anos, porque primeiro é preciso DESCOBRIR onde a criptografia é usada. Isso prepara a seção 4. Transição: temos um problema real, com prazo. Já sabemos resolver?");

// 6. NIST timeline
s = pres.addSlide({ masterName:"LIGHT" });
s.addText("3 · Já sabemos o que fazer? Os algoritmos existem",{placeholder:"title"});
const tl=[["2016","Chamada pública do NIST"],["2017","69 candidatos aceitos na 1ª rodada"],["2022","Primeiros algoritmos selecionados"],["2024","Padrões FIPS 203, 204 e 205"],["2025","HQC escolhido como reserva"]];
s.addShape(pres.shapes.LINE,{x:0.9,y:2.05,w:8.2,h:0,line:{color:"6C8EAD",width:2},objectName:"eixo"});
tl.forEach((t,i)=>{ const cx=0.9+i*2.05; const hi=(i===3);
  s.addShape(pres.shapes.OVAL,{x:cx-0.14,y:1.91,w:0.28,h:0.28,fill:{color:hi?"1FA89A":"FFFFFF"},line:{color:hi?"1FA89A":"6C8EAD",width:2},objectName:"marco"});
  T(s,t[0],{x:cx-0.9,y:1.4,w:1.8,h:0.4,fontSize:20,bold:true,color:hi?"1FA89A":"0F1B2D",fontFace:"Cambria",align:"center"});
  T(s,t[1],{x:cx-0.9,y:2.35,w:1.8,h:0.8,fontSize:14,color:"1F3A5F",align:"center",valign:"top"}); });
const pads=[["ML-KEM","FIPS 203","Troca de chaves"],["ML-DSA","FIPS 204","Assinaturas"],["SLH-DSA","FIPS 205","Assinaturas (baseada em hash)"]];
pads.forEach((p,i)=>{ const x=0.5+i*3.05; card(s,x,3.55,2.85,1.35,"EEF2F7","padrao",false);
  T(s,p[0],{x:x+0.25,y:3.65,w:2.4,h:0.4,fontSize:20,bold:true,color:"1FA89A",fontFace:"Cambria"});
  T(s,p[1]+" · "+p[2],{x:x+0.25,y:4.1,w:2.4,h:0.7,fontSize:14,color:"0F1B2D",valign:"top"}); });
s.addNotes("Seção 3 (3 min). Em 2016 o NIST abriu uma competição pública, aberta a pesquisadores do mundo todo, para encontrar algoritmos resistentes a computadores quânticos. Foram anos de rodadas e de ataques de criptoanalistas contra os candidatos. Em 2024 saíram os primeiros padrões: ML-KEM para troca de chaves, ML-DSA e SLH-DSA para assinaturas. ATENÇÃO: foi uma competição aberta, mas SEM premiação em dinheiro. Use 'competição' ou 'processo de padronização'.");

// 7. SIKE + gargalo
s = pres.addSlide({ masterName:"LIGHT" });
s.addText("Ter o padrão não é ter migrado",{placeholder:"title"});
card(s,0.5,1.5,4.4,3.4,"EEF2F7","sike",false);
T(s,"SIKE, 2022",{x:0.8,y:1.7,w:3.8,h:0.45,fontSize:22,bold:true,color:"E4572E",fontFace:"Cambria"});
T(s,"Chegou à 4ª rodada e foi quebrado por um computador comum em cerca de uma hora.",{x:0.8,y:2.25,w:3.8,h:1.1,fontSize:16,color:"0F1B2D",valign:"top"});
T(s,[{text:"O processo funciona: o erro apareceu antes do padrão.",options:{bullet:true,breakLine:true}},{text:"Algoritmos podem falhar: a migração precisa ser flexível (crypto-agility).",options:{bullet:true}}],{x:0.8,y:3.4,w:3.8,h:1.4,fontSize:14,color:"1F3A5F",valign:"top",paraSpaceAfter:6});
card(s,5.1,1.5,4.4,3.4,"0F1B2D","gargalo",false);
T(s,"O gargalo",{x:5.4,y:1.7,w:3.8,h:0.45,fontSize:22,bold:true,color:"F5A524",fontFace:"Cambria"});
T(s,"Falta saber onde trocar.",{x:5.4,y:2.25,w:3.8,h:0.8,fontSize:22,bold:true,color:"FFFFFF",fontFace:"Cambria",valign:"top"});
T(s,"O primeiro passo dos guias do NIST (NCCoE, SP 1800-38) é o inventário criptográfico: onde cada sistema usa criptografia e para quê. Em código real, isso é procurar à mão em milhares de arquivos.",{x:5.4,y:3.1,w:3.8,h:1.7,fontSize:14,color:"CADCFC",valign:"top"});
s.addNotes("O SIKE foi quebrado em 2022 em cerca de uma hora num computador comum. Mostra duas coisas: o processo funciona (erro descoberto antes da padronização) e a migração precisa de crypto-agility. Mas ter o padrão não significa ter migrado: o NCCoE (SP 1800-38) coloca o inventário criptográfico como primeiro passo. É esse o gargalo que o nosso trabalho ataca.");

// 8. Pipeline
s = pres.addSlide({ masterName:"DARK" });
s.addText("4 · A nossa solução: agentq",{placeholder:"title"});
T(s,"Um inventário criptográfico automático: onde está, para que serve e pelo que trocar.",{x:0.5,y:1.2,w:9,h:0.5,fontSize:18,color:"CADCFC"});
const pipe=[["Varrer","Respeita .gitignore e ignora dependências"],["Detectar","Árvore sintática (tree-sitter) com imports, apelidos e constantes"],["Inferir","Propósito: troca de chaves, cifragem ou assinatura"],["Avaliar","Base com 28 algoritmos e alternativas do NIST"],["Relatar","Mais urgente primeiro"]];
pipe.forEach((p,i)=>{ const x=0.5+i*1.82; card(s,x,2.1,1.65,2.5,"1F3A5F","etapa",false);
  num(s,x+0.15,2.25,0.45,String(i+1),"1FA89A","0F1B2D");
  T(s,p[0],{x:x+0.15,y:2.85,w:1.4,h:0.4,fontSize:18,bold:true,color:"FFFFFF",fontFace:"Cambria"});
  T(s,p[1],{x:x+0.15,y:3.3,w:1.4,h:1.2,fontSize:12,color:"CADCFC",valign:"top"}); });
T(s,"JavaScript, TypeScript e Python · escrito em Rust",{x:0.5,y:4.85,w:9,h:0.35,fontSize:13,color:"8FA6C0"});
s.addNotes("Seção 4 (6 min no total). Se a migração começa com um inventário, e fazer o inventário à mão é inviável, a automação é a primeira peça que falta. Fluxo: varre o projeto, detecta chamadas criptográficas com análise sintática (tree-sitter), infere o propósito de cada uso, avalia contra uma base de conhecimento de 28 algoritmos e gera o relatório ordenado por urgência. Hoje analisa JS/TS e Python; Java e Go estão previstos.");

// 9. Demo
s = pres.addSlide({ masterName:"LIGHT" });
s.addText("Demonstração: de um achado a uma ação",{placeholder:"title"});
card(s,0.5,1.45,5.6,3.5,"0F1B2D","achado",true);
s.addShape(pres.shapes.ROUNDED_RECTANGLE,{x:0.8,y:1.7,w:0.8,h:0.32,rectRadius:0.08,fill:{color:"E4572E"},line:{color:"E4572E",width:0},objectName:"selo"});
T(s,"ALTO",{x:0.8,y:1.7,w:0.8,h:0.32,fontSize:12,bold:true,color:"FFFFFF",align:"center",valign:"middle"});
T(s,"RSA (cifragem) · 2048 bits",{x:1.75,y:1.7,w:4.2,h:0.32,fontSize:15,bold:true,color:"FFFFFF",valign:"middle"});
T(s,"server.js:3",{x:0.8,y:2.15,w:5,h:0.3,fontSize:12,color:"8FA6C0",fontFace:"Courier New"});
T(s,"crypto.generateKeyPairSync('rsa', {",{x:0.8,y:2.55,w:5,h:0.35,fontSize:13,color:"3DDC97",fontFace:"Courier New"});
T(s,"A segurança depende de fatorar inteiros, que o Shor resolve. Tráfego gravado hoje pode ser decifrado depois (HNDL).",{x:0.8,y:3.05,w:5,h:0.9,fontSize:14,color:"CADCFC",valign:"top"});
T(s,[{text:"Trocar por: ",options:{bold:true,color:"F5A524"}},{text:"ML-KEM (CRYSTALS-Kyber)",options:{color:"FFFFFF",bold:true}}],{x:0.8,y:4.05,w:5,h:0.3,fontSize:15});
T(s,"@noble/post-quantum · OpenSSL 3.5+ · Bouncy Castle",{x:0.8,y:4.4,w:5,h:0.3,fontSize:12,color:"8FA6C0"});
const lab=[["Onde","Arquivo, linha e trecho de código"],["Por quê","Qual ameaça e qual urgência"],["Pelo quê","Substituto adequado ao propósito, com bibliotecas"]];
lab.forEach((l,i)=>{ const y=1.45+i*1.2; num(s,6.45,y,0.5,String(i+1),"1FA89A","FFFFFF");
  T(s,l[0],{x:7.15,y:y-0.02,w:2.4,h:0.35,fontSize:18,bold:true,color:"0F1B2D",fontFace:"Cambria"});
  T(s,l[1],{x:7.15,y:y+0.36,w:2.4,h:0.7,fontSize:14,color:"1F3A5F",valign:"top"}); });
s.addNotes("Demo (≈2 min). Abrir samples/node-shop no app desktop (cargo run --release --features gui --bin agentq-gui -- samples/node-shop) ou no terminal (cargo run -- samples/node-shop --tui). Mostre o painel (quantos usos vulneráveis, quais urgentes, quais arquivos revisar primeiro) e depois abra um achado RSA: linha do código, por que é risco, pelo que trocar. Conecte com a seção 2: isso cai em HNDL. O slide é um resumo caso a demo ao vivo falhe.");

// 10. Vantagens
s = pres.addSlide({ masterName:"LIGHT" });
s.addText("Por que o agentq ajuda desenvolvedores e analistas",{placeholder:"title"});
const adv=[["Análise sintática","Entende apelidos e constantes; ignora comentários e strings."],["Infere o propósito","Uma chave RSA pode cifrar, assinar ou trocar segredos: cada uso pede um substituto."],["Recomenda a troca","Algoritmo do NIST e bibliotecas que o implementam."],["Prioriza por urgência","O mais crítico aparece primeiro, no painel e no relatório."],["Para dev e analista","CLI e CI (SARIF, CBOM, --fail-on), terminal e app desktop com agendamento."],["Local e bilíngue","Roda na sua máquina, sem enviar o código. PT-BR e EN."]];
adv.forEach((a,i)=>{ const col=i%3,row=Math.floor(i/3); const x=0.5+col*3.05,y=1.45+row*1.8; card(s,x,y,2.85,1.6,"EEF2F7","vantagem",false);
  num(s,x+0.2,y+0.2,0.4,String(i+1),"1FA89A","FFFFFF");
  T(s,a[0],{x:x+0.75,y:y+0.2,w:1.95,h:0.4,fontSize:15,bold:true,color:"0F1B2D",fontFace:"Cambria",valign:"middle"});
  T(s,a[1],{x:x+0.2,y:y+0.72,w:2.45,h:0.85,fontSize:12,color:"1F3A5F",valign:"top"}); });
s.addNotes("Vantagens: (1) análise sintática, não busca de texto: entende const { createHash: h } = require('node:crypto'); h('md5'). (2) Infere propósito, o que muda a recomendação (ML-KEM ou ML-DSA). (3) Recomenda substituto e biblioteca. (4) Prioriza por urgência. (5) Serve a quem desenvolve (CLI, CI com --fail-on, SARIF para GitHub code scanning) e a quem audita (CBOM CycloneDX, app desktop com painel e varreduras agendadas). (6) Roda localmente, escrito em Rust, bilíngue.");

// 11. Resultados
s = pres.addSlide({ masterName:"LIGHT" });
s.addText("Resultados preliminares, com ressalvas",{placeholder:"title"});
s.addChart(pres.charts.BAR,[{name:"agentq",labels:["Precisão","Recall"],values:[100,97.1]},{name:"Busca por palavra-chave",labels:["Precisão","Recall"],values:[59.4,54.3]}],
 { x:0.5,y:1.4,w:5.4,h:3.6,barDir:"col",barGrouping:"clustered",chartColors:["1FA89A","6C8EAD"],showTitle:true,title:"Corpus anotado de Node.js (%)",titleFontSize:14,titleColor:"0F1B2D",titleFontFace:"+mn-lt",
  showValue:true,dataLabelPosition:"outEnd",dataLabelFontSize:12,dataLabelColor:"0F1B2D",dataLabelFontFace:"+mn-lt",dataLabelFormatCode:"0.0",showLegend:true,legendPos:"b",legendFontSize:12,legendColor:"1F3A5F",legendFontFace:"+mn-lt",
  catAxisLabelColor:"1F3A5F",catAxisLabelFontSize:12,catAxisLabelFontFace:"+mn-lt",valAxisLabelColor:"1F3A5F",valAxisLabelFontSize:11,valAxisLabelFontFace:"+mn-lt",
  valGridLine:{color:"D5DDE8",size:0.5},catGridLine:{style:"none"},valAxisMaxVal:120,valAxisMinVal:0 });
card(s,6.2,1.45,3.3,1.65,"FFF3DC","ressalva",false);
T(s,[{text:"Corpus escrito pelos autores",options:{bold:true,breakLine:true}},{text:"Serve como teste de regressão, não prova desempenho em código real.",options:{}}],{x:6.4,y:1.55,w:2.9,h:1.45,fontSize:14,color:"7A4B00",valign:"middle"});
card(s,6.2,3.3,3.3,1.65,"EEF2F7","qscan",false);
T(s,[{text:"Corpus independente (qScan)",options:{bold:true,breakLine:true}},{text:"agentq: 9 de 22 usos em JS. qScan: 21 de 22. Parte é escopo (TLS, SSH, dependências), parte é critério.",options:{}}],{x:6.4,y:3.4,w:2.9,h:1.45,fontSize:13,color:"0F1B2D",valign:"middle"});
s.addNotes("Números do README: precisão 100% e recall 97,1% do agentq contra 59,4% e 54,3% da busca por palavra-chave, no corpus anotado de Node. SEJA HONESTO: o corpus foi escrito pelos autores, serve como teste de regressão. No corpus independente do qScan (ferramenta léxica existente), o agentq achou 9 de 22 usos em JS contra 21 de 22 do qScan; parte é escopo (configuração TLS, chaves SSH, dependências) e parte é critério: o qScan reporta qualquer menção ao nome do algoritmo, o agentq só chamadas reais. Os números de 9/22 foram medidos antes de TLS e manifestos de dependências serem suportados: atualize se rodar de novo. Não diga que é a primeira ferramenta do tipo: qScan e pqcheck já existem. O diferencial a demonstrar é precisão e propósito nos casos difíceis.");

// 12. Fechamento
s = pres.addSlide({ masterName:"DARK" });
s.addText("Limitações e próximos passos",{placeholder:"title"});
card(s,0.5,1.45,4.4,2.55,"1F3A5F","limites",false);
T(s,"Hoje",{x:0.8,y:1.6,w:3.8,h:0.4,fontSize:20,bold:true,color:"F5A524",fontFace:"Cambria"});
T(s,[{text:"JavaScript, TypeScript e Python",options:{bullet:true,breakLine:true}},{text:"Nomes resolvidos dentro de um arquivo; algoritmos escolhidos em tempo de execução não aparecem",options:{bullet:true,breakLine:true}},{text:"Validado em corpus sintético",options:{bullet:true}}],{x:0.8,y:2.05,w:3.8,h:1.85,fontSize:14,color:"FFFFFF",valign:"top",paraSpaceAfter:6});
card(s,5.1,1.45,4.4,2.55,"1F3A5F","proximos",false);
T(s,"Próximos passos",{x:5.4,y:1.6,w:3.8,h:0.4,fontSize:20,bold:true,color:"3DDC97",fontFace:"Cambria"});
T(s,[{text:"Validar em repositórios reais com conferência manual",options:{bullet:true,breakLine:true}},{text:"Comparar com ferramentas existentes",options:{bullet:true,breakLine:true}},{text:"Analisadores para Java e Go",options:{bullet:true}}],{x:5.4,y:2.05,w:3.8,h:1.85,fontSize:14,color:"FFFFFF",valign:"top",paraSpaceAfter:6});
T(s,"A migração leva anos e o prazo corre desde que alguém começou a gravar. O primeiro passo é saber o que temos.",{x:0.5,y:4.25,w:9,h:0.7,fontSize:18,color:"CADCFC",fontFace:"Cambria",valign:"top"});
s.addNotes("Fechamento (1 min). A migração pós-quântica vai levar anos, e o prazo corre desde que alguém começou a gravar tráfego. O primeiro passo é saber o que temos. O agentq é uma contribuição para esse primeiro passo. Obrigado, ficamos à disposição para perguntas. Respostas prontas: (1) já existem ferramentas? Sim, qScan e pqcheck; a diferença está na análise sintática e na inferência de propósito. (2) números valem para código real? Não, por isso o slide de ressalvas. (3) E se o computador quântico nunca chegar? A migração também elimina MD5, SHA-1 e fraquezas clássicas, que a ferramenta já aponta. (4) E a simétrica? Grover dá ganho só quadrático, AES-256 basta; o foco é chave pública.");

(async()=>{ await pres.writeFile({fileName:"agentq-apresentacao.pptx"}); await applyTheme("agentq-apresentacao.pptx", THEME); console.log("ok"); })();
