import { createFileRoute, Link } from "@tanstack/react-router";
import { useEffect, useMemo, useRef, useState } from "react";

export const Route = createFileRoute("/dashboard")({
  component: Dashboard,
  head: () => ({ meta: [{ title: "Dashboard — StargazeMPP" }] }),
});

type Provider = {
  id: string;
  name: string;
  category: string;
  rail: string;
  price: string;
  rep: number;
  p95: string;
  status: "online" | "degraded";
  about: string;
};

const PROVIDERS: Provider[] = [
  { id: "mpp32", name: "mpp32", category: "Generalist intelligence", rail: "tempo + solana", price: "8000 PathUSD/q", rep: 942, p95: "1.4s", status: "online", about: "Broad multi-modal reasoning with rich JSON schemas." },
  { id: "stargaze-signals", name: "Stargaze Tech · Signals", category: "Financial signals", rail: "tempo", price: "variable", rep: 891, p95: "240ms", status: "online", about: "Real-time market and risk scoring tuned for on-chain agents, by Stargaze Tech." },
  { id: "stargaze-geo", name: "Stargaze Tech · Geo", category: "Geo-verified data", rail: "solana", price: "ZK-verified", rep: 877, p95: "620ms", status: "degraded", about: "Location-bounded queries verified by Light Protocol Groth16 proofs, by Stargaze Tech." },
];

const NAV = [
  { id: "overview", label: "Overview", icon: "M3 12l9-9 9 9M5 10v10h14V10" },
  { id: "providers", label: "Providers", icon: "M3 7h18M3 12h18M3 17h18" },
  { id: "playground", label: "Playground", icon: "M8 5v14l11-7z" },
  { id: "sessions", label: "Sessions", icon: "M12 6v6l4 2M12 22a10 10 0 110-20 10 10 0 010 20z" },
  { id: "stake", label: "Stake & Earn", icon: "M12 2v20M2 12h20" },
  { id: "reputation", label: "Reputation", icon: "M9 12l2 2 4-4m5 2a9 9 0 11-18 0 9 9 0 0118 0z" },
  { id: "docs", label: "Docs", icon: "M4 4h16v16H4z" },
];

type Wallet = { name: "Phantom" | "Solflare"; publicKey: string };
type Session = { id: string; prov: string; rail: string; cum: number; st: "open" | "settled" };
type Activity = { t: string; label: string; meta: string };

function shortKey(k: string) { return k ? k.slice(0, 4) + "…" + k.slice(-4) : ""; }
function nowHHMM() { const d = new Date(); return d.toTimeString().slice(0, 5); }

function Dashboard() {
  const [tab, setTab] = useState("overview");
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const [mobileNavOpen, setMobileNavOpen] = useState(false);

  // Close mobile drawer on tab change or resize to desktop
  useEffect(() => { setMobileNavOpen(false); }, [tab]);
  useEffect(() => {
    const onResize = () => { if (window.innerWidth >= 1024) setMobileNavOpen(false); };
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);
  useEffect(() => {
    document.body.style.overflow = mobileNavOpen ? "hidden" : "";
    return () => { document.body.style.overflow = ""; };
  }, [mobileNavOpen]);

  useEffect(() => {
    const valid = new Set(["overview","providers","playground","sessions","stake","reputation","docs"]);
    const apply = () => {
      const h = window.location.hash.replace("#","");
      if (h && valid.has(h)) setTab(h);
    };
    apply();
    window.addEventListener("hashchange", apply);
    return () => window.removeEventListener("hashchange", apply);
  }, []);

  useEffect(() => {
    const c = canvasRef.current; if (!c) return;
    const ctx = c.getContext("2d"); if (!ctx) return;
    const DPR = Math.min(window.devicePixelRatio || 1, 2);
    let W = 0, H = 0, raf = 0, lastMeteor = 0;
    const stars: any[] = [], meteors: any[] = [];
    const mouse = { x: 0, y: 0, tx: 0, ty: 0 };
    function resize() {
      W = c!.clientWidth; H = c!.clientHeight;
      c!.width = W * DPR; c!.height = H * DPR;
      ctx!.setTransform(DPR, 0, 0, DPR, 0, 0);
      stars.length = 0;
      const n = Math.min(160, Math.floor((W * H) / 10000));
      for (let i = 0; i < n; i++) stars.push({
        x: Math.random()*W, y: Math.random()*H, z: Math.random()*0.8+0.2,
        r: Math.random()*1.6+0.4, vx:(Math.random()-.5)*0.05, vy:(Math.random()-.5)*0.05,
        tw: Math.random()*Math.PI*2, hue: 260+Math.random()*40
      });
    }
    function tick(t: number) {
      ctx!.clearRect(0,0,W,H);
      mouse.x += (mouse.tx - mouse.x)*0.04; mouse.y += (mouse.ty - mouse.y)*0.04;
      for (const s of stars) {
        s.x += s.vx + mouse.x*0.4*s.z; s.y += s.vy + mouse.y*0.4*s.z;
        if (s.x<-10) s.x=W+10; if (s.x>W+10) s.x=-10;
        if (s.y<-10) s.y=H+10; if (s.y>H+10) s.y=-10;
        s.tw += 0.02 + s.z*0.04;
        const a = 0.55 + Math.sin(s.tw)*0.45;
        const g = ctx!.createRadialGradient(s.x,s.y,0,s.x,s.y,s.r*6);
        g.addColorStop(0,`hsla(${s.hue},90%,85%,${a})`); g.addColorStop(1,`hsla(${s.hue},90%,70%,0)`);
        ctx!.fillStyle = g; ctx!.beginPath(); ctx!.arc(s.x,s.y,s.r*6,0,Math.PI*2); ctx!.fill();
        ctx!.fillStyle = `rgba(255,255,255,${a})`; ctx!.beginPath(); ctx!.arc(s.x,s.y,s.r,0,Math.PI*2); ctx!.fill();
      }
      const maxD = 130;
      for (let i=0;i<stars.length;i++) for (let j=i+1;j<stars.length;j++) {
        const a=stars[i], b=stars[j], dx=a.x-b.x, dy=a.y-b.y, d=Math.hypot(dx,dy);
        if (d<maxD) {
          let alpha=(1-d/maxD)*0.18*Math.min(a.z,b.z);
          const mx=mouse.tx*W*0.5+W/2, my=mouse.ty*H*0.5+H/2;
          const md=Math.hypot((a.x+b.x)/2-mx,(a.y+b.y)/2-my);
          if (md<200) alpha += (1-md/200)*0.35;
          ctx!.strokeStyle=`rgba(196,179,236,${alpha})`; ctx!.lineWidth=0.6;
          ctx!.beginPath(); ctx!.moveTo(a.x,a.y); ctx!.lineTo(b.x,b.y); ctx!.stroke();
        }
      }
      if (t-lastMeteor>1800 && Math.random()<0.6) {
        meteors.push({ x:Math.random()*W*0.6, y:-20, vx:6+Math.random()*4, vy:3+Math.random()*2, life:0, max:90+Math.random()*50, hue:270+Math.random()*40 });
        lastMeteor=t;
      }
      for (let k=meteors.length-1;k>=0;k--) {
        const m=meteors[k]; m.x+=m.vx; m.y+=m.vy; m.life++;
        const tx=m.x-m.vx*12, ty=m.y-m.vy*12;
        const lg=ctx!.createLinearGradient(tx,ty,m.x,m.y);
        lg.addColorStop(0,`hsla(${m.hue},100%,80%,0)`); lg.addColorStop(1,`hsla(${m.hue},100%,90%,0.95)`);
        ctx!.strokeStyle=lg; ctx!.lineWidth=2; ctx!.lineCap="round";
        ctx!.beginPath(); ctx!.moveTo(tx,ty); ctx!.lineTo(m.x,m.y); ctx!.stroke();
        const hg=ctx!.createRadialGradient(m.x,m.y,0,m.x,m.y,12);
        hg.addColorStop(0,"rgba(255,255,255,0.9)"); hg.addColorStop(1,`hsla(${m.hue},100%,70%,0)`);
        ctx!.fillStyle=hg; ctx!.beginPath(); ctx!.arc(m.x,m.y,12,0,Math.PI*2); ctx!.fill();
        if (m.life>m.max||m.x>W+50||m.y>H+50) meteors.splice(k,1);
      }
      raf = requestAnimationFrame(tick);
    }
    function onMove(e: MouseEvent) {
      mouse.tx = e.clientX/window.innerWidth - 0.5;
      mouse.ty = e.clientY/window.innerHeight - 0.5;
      document.querySelectorAll<HTMLElement>(".dcosmos .parallax").forEach(p => {
        const d = parseFloat(p.dataset.depth || "1");
        p.style.transform = `translate3d(${-mouse.tx*20*d}px, ${-mouse.ty*20*d}px, 0)`;
      });
    }
    resize();
    window.addEventListener("resize", resize);
    window.addEventListener("mousemove", onMove);
    raf = requestAnimationFrame(tick);
    return () => { cancelAnimationFrame(raf); window.removeEventListener("resize", resize); window.removeEventListener("mousemove", onMove); };
  }, []);

  const [selected, setSelected] = useState<Provider>(PROVIDERS[0]);
  const [query, setQuery] = useState('{\n  "prompt": "Summarize today\'s on-chain volume."\n}');
  const [response, setResponse] = useState<string | null>(null);
  const [signing, setSigning] = useState(false);

  const [wallet, setWallet] = useState<Wallet | null>(null);
  const [walletOpen, setWalletOpen] = useState(false);
  const [walletErr, setWalletErr] = useState<string | null>(null);
  const [walletBusy, setWalletBusy] = useState<string | null>(null);

  const [sessions, setSessions] = useState<Session[]>([
    { id: "sess_01HX9P3K", prov: "mpp32", rail: "tempo", cum: 32000, st: "open" },
    { id: "sess_01HX9N12", prov: "stargaze-signals", rail: "tempo", cum: 8400, st: "open" },
    { id: "sess_01HX8B45", prov: "stargaze-geo", rail: "solana", cum: 4200, st: "settled" },
  ]);
  const [escrow, setEscrow] = useState(42000);
  const [queriesEpoch, setQueriesEpoch] = useState(186);
  const [repEarned, setRepEarned] = useState(128);
  const [activity, setActivity] = useState<Activity[]>([
    { t: "12:04", label: "voucher signed", meta: "mpp32 · 8000" },
    { t: "11:58", label: "session opened", meta: "stargaze-signals · 20,000 escrow" },
    { t: "11:42", label: "settled on-chain", meta: "stargaze-geo · 4,200" },
    { t: "11:10", label: "reputation refresh", meta: "epoch 47 snapshot" },
  ]);

  const [stakeProvider, setStakeProvider] = useState(PROVIDERS[0].id);
  const [stakeAmount, setStakeAmount] = useState("");
  const [staked, setStaked] = useState(12400);
  const [rewards, setRewards] = useState(184.2);

  const [toast, setToast] = useState<string | null>(null);
  const showToast = (msg: string) => { setToast(msg); setTimeout(() => setToast(null), 2400); };

  // Restore wallet from localStorage
  useEffect(() => {
    try {
      const raw = localStorage.getItem("stargaze_wallet");
      if (raw) setWallet(JSON.parse(raw));
    } catch {}
  }, []);
  useEffect(() => {
    if (wallet) localStorage.setItem("stargaze_wallet", JSON.stringify(wallet));
    else localStorage.removeItem("stargaze_wallet");
  }, [wallet]);

  const requireWallet = (action: string) => {
    if (!wallet) {
      setWalletOpen(true);
      showToast("Connect a wallet to " + action);
      return false;
    }
    return true;
  };

  const connectPhantom = async () => {
    setWalletErr(null); setWalletBusy("Phantom");
    try {
      const provider = (window as any)?.phantom?.solana || (window as any)?.solana;
      if (!provider || !provider.isPhantom) {
        window.open("https://phantom.app/download", "_blank");
        throw new Error("Phantom extension not detected. Install it and reload.");
      }
      const res = await provider.connect();
      const pk = res?.publicKey?.toString?.() || res?.publicKey || "";
      setWallet({ name: "Phantom", publicKey: pk });
      setWalletOpen(false);
      showToast("Phantom connected · " + shortKey(pk));
    } catch (e: any) {
      setWalletErr(e?.message || "Failed to connect Phantom");
    } finally { setWalletBusy(null); }
  };

  const connectSolflare = async () => {
    setWalletErr(null); setWalletBusy("Solflare");
    try {
      const provider = (window as any)?.solflare;
      if (!provider) {
        window.open("https://solflare.com/download", "_blank");
        throw new Error("Solflare extension not detected. Install it and reload.");
      }
      await provider.connect();
      const pk = provider.publicKey?.toString?.() || "";
      if (!pk) throw new Error("Could not read public key");
      setWallet({ name: "Solflare", publicKey: pk });
      setWalletOpen(false);
      showToast("Solflare connected · " + shortKey(pk));
    } catch (e: any) {
      setWalletErr(e?.message || "Failed to connect Solflare");
    } finally { setWalletBusy(null); }
  };

  const disconnect = async () => {
    try {
      const p: any = wallet?.name === "Phantom"
        ? ((window as any)?.phantom?.solana || (window as any)?.solana)
        : (window as any)?.solflare;
      await p?.disconnect?.();
    } catch {}
    setWallet(null);
    showToast("Wallet disconnected");
  };

  const runQuery = () => {
    if (!requireWallet("sign vouchers")) return;
    setSigning(true);
    setResponse(null);
    setTimeout(() => {
      const cost = selected.id === "mpp32" ? 8000 : selected.id === "stargaze-signals" ? 1200 : 4200;
      const sid = "sess_" + Math.random().toString(36).slice(2, 12).toUpperCase();
      const next: Session = { id: sid.slice(0, 12), prov: selected.id, rail: selected.rail.split(" ")[0], cum: cost, st: "open" };
      setSessions(s => [next, ...s]);
      setEscrow(e => Math.max(0, e - cost));
      setQueriesEpoch(q => q + 1);
      setRepEarned(r => r + 2);
      setActivity(a => [{ t: nowHHMM(), label: "voucher signed", meta: `${selected.id} · ${cost}` }, ...a].slice(0, 8));
      setResponse(JSON.stringify({
        sessionId: next.id,
        rail: next.rail,
        cumulative: String(cost),
        nonce: 1,
        wallet: shortKey(wallet!.publicKey),
        result: { ok: true, summary: "Mocked agent response — voucher signed by " + wallet!.name + "." }
      }, null, 2));
      setSigning(false);
      showToast("Voucher signed · " + cost + " PathUSD");
    }, 900);
  };

  const closeSession = (id: string) => {
    setSessions(s => s.map(x => x.id === id ? { ...x, st: "settled" } : x));
    const sess = sessions.find(s => s.id === id);
    if (sess) {
      setEscrow(e => e + Math.floor(sess.cum * 0.1));
      setActivity(a => [{ t: nowHHMM(), label: "settled on-chain", meta: `${sess.prov} · ${sess.cum.toLocaleString()}` }, ...a].slice(0, 8));
      showToast("Session settled · " + sess.id);
    }
  };

  const openNewSession = (provId: string) => {
    if (!requireWallet("open a session")) return;
    const sid = "sess_" + Math.random().toString(36).slice(2, 10).toUpperCase();
    const prov = PROVIDERS.find(p => p.id === provId)!;
    setSessions(s => [{ id: sid, prov: provId, rail: prov.rail.split(" ")[0], cum: 0, st: "open" }, ...s]);
    setEscrow(e => e + 20000);
    setActivity(a => [{ t: nowHHMM(), label: "session opened", meta: `${provId} · 20,000 escrow` }, ...a].slice(0, 8));
    showToast("Session opened with " + provId);
  };

  const doStake = () => {
    if (!requireWallet("stake")) return;
    const amt = parseFloat(stakeAmount);
    if (!amt || amt <= 0) { showToast("Enter an amount"); return; }
    setStaked(s => s + amt);
    setStakeAmount("");
    setActivity(a => [{ t: nowHHMM(), label: "staked", meta: `${stakeProvider} · ${amt.toLocaleString()}` }, ...a].slice(0, 8));
    showToast("Staked " + amt.toLocaleString() + " on " + stakeProvider);
  };

  const claimRewards = () => {
    if (!requireWallet("claim")) return;
    if (rewards <= 0) { showToast("Nothing to claim"); return; }
    const r = rewards;
    setRewards(0);
    setActivity(a => [{ t: nowHHMM(), label: "rewards claimed", meta: `+${r.toFixed(2)}` }, ...a].slice(0, 8));
    showToast("Claimed " + r.toFixed(2) + " PathUSD");
  };

  const activeSessions = useMemo(() => sessions.filter(s => s.st === "open").length, [sessions]);

  return (
    <div className="min-h-screen text-[#ebe6f7] relative" style={{ background: "#0e0820", fontFamily: "Inter Tight, system-ui, sans-serif" }}>
      <style>{`
        .ds-card { background: linear-gradient(180deg, rgba(196,179,236,0.06), rgba(196,179,236,0.02)); border: 1px solid rgba(196,179,236,0.16); border-radius: 14px; }
        .ds-input { background: rgba(196,179,236,0.04); border: 1px solid rgba(196,179,236,0.16); color: #ebe6f7; border-radius: 10px; }
        .ds-input:focus { outline: none; border-color: #c4b3ec; }
        .ds-mono { font-family: 'JetBrains Mono', ui-monospace, monospace; }
        .ds-display { font-family: 'Fraunces', Georgia, serif; letter-spacing: -0.02em; }
        .ds-pill { display:inline-flex; align-items:center; gap:6px; padding:3px 10px; border-radius:999px; font-size:11px; }
        .ds-btn { padding:8px 16px; border-radius:9999px; font-size:13px; font-weight:500; transition: opacity .2s; }
        .ds-btn:disabled { opacity:.55; cursor:not-allowed; }
        .ds-modal { position:fixed; inset:0; z-index:60; display:flex; align-items:center; justify-content:center; background:rgba(8,5,18,0.72); backdrop-filter: blur(8px); animation: fade .2s ease; }
        @keyframes fade { from { opacity: 0 } to { opacity: 1 } }
        .ds-toast { position: fixed; bottom: 24px; left: 50%; transform: translateX(-50%); z-index: 70; background: #c4b3ec; color: #1a1530; padding: 10px 18px; border-radius: 999px; font-size: 13px; font-weight: 500; box-shadow: 0 12px 40px -10px rgba(196,179,236,0.4); }
        .dcosmos { position: fixed; inset: 0; z-index: 0; pointer-events: none; overflow: hidden;
          background:
            radial-gradient(ellipse at 15% 0%, #2c1f5e 0%, transparent 55%),
            radial-gradient(ellipse at 85% 100%, #3a2773 0%, transparent 55%),
            linear-gradient(180deg, #120c28 0%, #0e0820 100%);
        }
        .dcosmos .stars, .dcosmos .stars2, .dcosmos .stars3 { position:absolute; inset:-50%; background-repeat: repeat; }
        .dcosmos .stars { background-image:
            radial-gradient(1.2px 1.2px at 20px 30px, #fff, transparent 60%),
            radial-gradient(1px 1px at 80px 120px, #e9defb, transparent 60%),
            radial-gradient(1px 1px at 160px 60px, #fff, transparent 60%),
            radial-gradient(1.4px 1.4px at 240px 200px, #c4b3ec, transparent 60%),
            radial-gradient(1px 1px at 320px 90px, #fff, transparent 60%),
            radial-gradient(1px 1px at 50px 260px, #d6c8f4, transparent 60%);
          background-size: 380px 380px; opacity: .9; animation: dr1 180s linear infinite; }
        .dcosmos .stars2 { background-image:
            radial-gradient(1.6px 1.6px at 40px 80px, #fff, transparent 60%),
            radial-gradient(1px 1px at 200px 40px, #c4b3ec, transparent 60%),
            radial-gradient(1.2px 1.2px at 360px 220px, #fff, transparent 60%),
            radial-gradient(1px 1px at 120px 320px, #b8a4e8, transparent 60%);
          background-size: 520px 520px; opacity: .65; animation: dr2 260s linear infinite, dtw 6s ease-in-out infinite; }
        .dcosmos .stars3 { background-image:
            radial-gradient(2px 2px at 100px 150px, #fff, transparent 60%),
            radial-gradient(1.6px 1.6px at 380px 60px, #d6c8f4, transparent 60%),
            radial-gradient(2.4px 2.4px at 260px 400px, #c4b3ec, transparent 60%);
          background-size: 700px 700px; opacity: .7; animation: dr3 360s linear infinite, dtw 4s ease-in-out infinite alternate;
          filter: drop-shadow(0 0 4px rgba(196,179,236,0.6)); }
        .dcosmos .neb { position:absolute; border-radius:50%; filter: blur(90px); will-change: transform; }
        .dcosmos .neb.a { width:70vmax; height:70vmax; top:-25%; left:-20%; background: radial-gradient(circle, rgba(139,108,232,0.55), transparent 65%); animation: dfa 38s ease-in-out infinite; }
        .dcosmos .neb.b { width:60vmax; height:60vmax; bottom:-25%; right:-20%; background: radial-gradient(circle, rgba(94,62,184,0.5), transparent 65%); animation: dfb 46s ease-in-out infinite; }
        .dcosmos .neb.c { width:45vmax; height:45vmax; top:30%; left:45%; background: radial-gradient(circle, rgba(196,179,236,0.32), transparent 70%); animation: dfc 52s ease-in-out infinite; }
        @keyframes dr1 { from { transform: translate3d(0,0,0); } to { transform: translate3d(-380px,-380px,0); } }
        @keyframes dr2 { from { transform: translate3d(0,0,0); } to { transform: translate3d(520px,-520px,0); } }
        @keyframes dr3 { from { transform: translate3d(0,0,0); } to { transform: translate3d(-700px,700px,0); } }
        @keyframes dtw { 0%,100% { filter: brightness(1); } 50% { filter: brightness(1.6); } }
        @keyframes dfa { 0%,100% { transform: translate(0,0) scale(1); } 50% { transform: translate(8vmax,6vmax) scale(1.08); } }
        @keyframes dfb { 0%,100% { transform: translate(0,0) scale(1); } 50% { transform: translate(-10vmax,-6vmax) scale(1.1); } }
        @keyframes dfc { 0%,100% { transform: translate(-50%,-50%) scale(1); } 50% { transform: translate(-30%,-70%) scale(1.15); } }
        .dcosmos .aurora { position:absolute; inset:-25%; pointer-events:none; mix-blend-mode: screen; opacity:.55; filter: blur(60px) saturate(140%);
          background:
            conic-gradient(from 0deg at 30% 40%, transparent 0deg, rgba(167,139,250,.55) 60deg, transparent 140deg, rgba(94,62,184,.45) 220deg, transparent 300deg),
            conic-gradient(from 180deg at 70% 60%, transparent 0deg, rgba(216,180,254,.4) 80deg, transparent 160deg, rgba(124,58,237,.5) 260deg, transparent 340deg);
          animation: daur 60s linear infinite; }
        .dcosmos .aurora.b { opacity:.35; animation: daurr 90s linear infinite; filter: blur(80px) saturate(160%); }
        @keyframes daur { to { transform: rotate(360deg) scale(1.05); } }
        @keyframes daurr { to { transform: rotate(-360deg) scale(1.1); } }
        .dcosmos .grid-floor { position:absolute; left:50%; bottom:-30vh; width:240vw; height:80vh; transform: translateX(-50%) perspective(700px) rotateX(62deg); transform-origin: 50% 0%;
          background-image:
            linear-gradient(rgba(196,179,236,0.18) 1px, transparent 1px),
            linear-gradient(90deg, rgba(196,179,236,0.18) 1px, transparent 1px);
          background-size: 80px 80px;
          mask-image: linear-gradient(180deg, transparent 0%, #000 35%, #000 70%, transparent 100%);
          -webkit-mask-image: linear-gradient(180deg, transparent 0%, #000 35%, #000 70%, transparent 100%);
          animation: dgp 14s linear infinite; opacity:.45; }
        @keyframes dgp { from { background-position: 0 0, 0 0; } to { background-position: 0 80px, 0 0; } }
        .dcosmos .orb { position:absolute; border-radius:50%; pointer-events:none; mix-blend-mode: screen; filter: blur(2px);
          background: radial-gradient(circle at 35% 35%, #fff, #c4b3ec 30%, rgba(124,58,237,.6) 60%, transparent 75%);
          box-shadow: 0 0 40px rgba(167,139,250,.6), 0 0 90px rgba(124,58,237,.45); }
        .dcosmos .orb.o1 { width:14px; height:14px; top:18%; left:12%; animation: dof1 22s ease-in-out infinite; }
        .dcosmos .orb.o2 { width:9px; height:9px; top:62%; left:78%; animation: dof2 28s ease-in-out infinite; }
        .dcosmos .orb.o3 { width:18px; height:18px; top:78%; left:22%; animation: dof3 34s ease-in-out infinite; }
        .dcosmos .orb.o4 { width:7px; height:7px; top:30%; left:62%; animation: dof1 26s ease-in-out infinite reverse; }
        @keyframes dof1 { 0%,100%{ transform: translate(0,0);} 50%{ transform: translate(30vw,-18vh);} }
        @keyframes dof2 { 0%,100%{ transform: translate(0,0);} 50%{ transform: translate(-26vw,22vh);} }
        @keyframes dof3 { 0%,100%{ transform: translate(0,0);} 50%{ transform: translate(18vw,-30vh);} }
        .dcosmos .vignette { position:absolute; inset:0; pointer-events:none; background: radial-gradient(ellipse at center, transparent 40%, rgba(8,4,20,.7) 100%); }
        .dcosmos canvas { position:absolute; inset:0; width:100%; height:100%; display:block; }
        .dcosmos .parallax { position:absolute; inset:0; transition: transform 1.2s cubic-bezier(.2,.8,.2,1); will-change: transform; }
        .ds-shell { position: relative; z-index: 1; }
      `}</style>

      <div className="dcosmos" aria-hidden="true">
        <div className="parallax" data-depth="0.4">
          <div className="aurora"></div>
          <div className="aurora b"></div>
          <div className="neb a"></div>
          <div className="neb b"></div>
          <div className="neb c"></div>
        </div>
        <div className="parallax" data-depth="0.8">
          <div className="stars"></div>
          <div className="stars2"></div>
          <div className="stars3"></div>
        </div>
        <canvas ref={canvasRef}></canvas>
        <div className="parallax" data-depth="1.6">
          <div className="orb o1"></div><div className="orb o2"></div><div className="orb o3"></div><div className="orb o4"></div>
        </div>
        <div className="vignette"></div>
      </div>

      <div className="flex ds-shell">
        {/* SIDEBAR */}
        <aside className="hidden lg:flex w-64 shrink-0 flex-col border-r min-h-screen p-5" style={{ borderColor: "rgba(196,179,236,0.16)" }}>
          <Link to="/" className="flex items-center gap-2.5 mb-10">
            <img src="/logo.png" alt="" className="w-8 h-8 object-contain"/>
            <span className="ds-display text-[17px]">StargazeMPP</span>
          </Link>
          <nav className="flex flex-col gap-1">
            {NAV.map(n => (
              <button key={n.id} onClick={() => n.id === "docs" ? (window.location.href = "/docs") : setTab(n.id)}
                className={`flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm transition-colors ${tab === n.id ? "text-[#1a1530]" : "text-[#a89fc7] hover:text-[#ebe6f7]"}`}
                style={tab === n.id ? { background: "#c4b3ec" } : {}}>
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d={n.icon}/></svg>
                {n.label}
              </button>
            ))}
          </nav>
          <div className="mt-auto ds-card p-4">
            <p className="ds-mono text-[10px] uppercase tracking-widest text-[#a89fc7]">Wallet</p>
            {wallet ? (
              <>
                <p className="text-sm mt-1">{shortKey(wallet.publicKey)}</p>
                <p className="ds-mono text-[10px] mt-2 text-[#a89fc7]">{wallet.name} · linked</p>
                <button onClick={disconnect} className="mt-3 text-[11px] ds-mono text-[#c4b3ec] hover:underline">disconnect →</button>
              </>
            ) : (
              <>
                <p className="text-sm mt-1 text-[#a89fc7]">Not connected</p>
                <button onClick={() => setWalletOpen(true)} className="mt-3 ds-btn w-full" style={{ background: "#c4b3ec", color: "#1a1530" }}>Connect wallet</button>
              </>
            )}
          </div>
        </aside>

        {/* MAIN */}
        <main className="flex-1 min-w-0">
          {/* topbar */}
          <div className="flex items-center justify-between border-b px-6 py-4 sticky top-0 backdrop-blur-md z-10 gap-3" style={{ borderColor: "rgba(196,179,236,0.16)", background: "rgba(26,21,48,0.7)" }}>
            <div className="flex items-center gap-3 min-w-0">
              <button
                onClick={() => setMobileNavOpen(true)}
                className="lg:hidden p-2 -ml-2 rounded-lg text-[#ebe6f7]"
                aria-label="Open navigation"
                aria-controls="ds-mobile-nav"
                aria-expanded={mobileNavOpen}
              >
                <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M3 6h18M3 12h18M3 18h18"/></svg>
              </button>
              <Link to="/" className="lg:hidden flex items-center gap-2"><img src="/logo.png" className="w-7 h-7" alt=""/></Link>
              <h1 className="ds-display text-xl sm:text-2xl capitalize truncate">{NAV.find(n => n.id === tab)?.label}</h1>
            </div>
            <div className="flex items-center gap-3">
              <span className="hidden sm:inline-flex ds-pill ds-mono uppercase tracking-widest" style={{ background: "rgba(196,179,236,0.1)", color: "#c4b3ec" }}>
                <span className="w-1.5 h-1.5 rounded-full bg-[#c4b3ec]"/> Mainnet
              </span>
              {wallet ? (
                <button onClick={() => setWalletOpen(true)} className="ds-btn flex items-center gap-2 whitespace-nowrap" style={{ background: "rgba(196,179,236,0.12)", color: "#ebe6f7", border: "1px solid rgba(196,179,236,0.32)" }}>
                  <span className="w-2 h-2 rounded-full bg-[#c4b3ec]"/>
                  <span className="ds-mono text-[12px]">{shortKey(wallet.publicKey)}</span>
                </button>
              ) : (
                <button onClick={() => setWalletOpen(true)} className="ds-btn whitespace-nowrap" style={{ background: "#c4b3ec", color: "#1a1530" }}>
                  <span className="hidden sm:inline">Connect wallet</span>
                  <span className="sm:hidden">Connect</span>
                </button>
              )}
            </div>
          </div>

          <div className="p-4 sm:p-6 lg:p-10 max-w-[1400px]">
            {tab === "overview" && (
              <div className="grid gap-5">
                <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-4">
                  {[
                    { l: "Active sessions", v: String(activeSessions), s: sessions.length + " total" },
                    { l: "Escrow balance", v: escrow.toLocaleString(), s: "PathUSD" },
                    { l: "Queries this epoch", v: String(queriesEpoch), s: "p95 1.1s" },
                    { l: "Reputation earned", v: "+" + repEarned, s: "score delta" },
                  ].map(s => (
                    <div key={s.l} className="ds-card p-5">
                      <p className="ds-mono text-[10px] uppercase tracking-widest text-[#a89fc7]">{s.l}</p>
                      <p className="ds-display text-4xl mt-2">{s.v}</p>
                      <p className="ds-mono text-[11px] text-[#a89fc7] mt-2">{s.s}</p>
                    </div>
                  ))}
                </div>
                <div className="grid lg:grid-cols-3 gap-4">
                  <div className="ds-card p-5 lg:col-span-2">
                    <p className="ds-mono text-[10px] uppercase tracking-widest text-[#a89fc7] mb-3">Recent activity</p>
                    <div className="divide-y" style={{ borderColor: "rgba(196,179,236,0.16)" }}>
                      {activity.map((r, i) => (
                        <div key={i} className="flex items-center justify-between py-3 text-sm">
                          <span className="ds-mono text-[#a89fc7] w-12">{r.t}</span>
                          <span className="flex-1 ml-4">{r.label}</span>
                          <span className="ds-mono text-xs text-[#c4b3ec]">{r.meta}</span>
                        </div>
                      ))}
                    </div>
                  </div>
                  <div className="ds-card p-5">
                    <p className="ds-mono text-[10px] uppercase tracking-widest text-[#a89fc7]">Routing fees (epoch)</p>
                    <p className="ds-display text-5xl mt-3">2.41%</p>
                    <p className="text-sm text-[#a89fc7] mt-2">200 bps total. 50% burned, 50% to stakers.</p>
                    <div className="mt-4 h-2 rounded-full overflow-hidden" style={{ background: "rgba(196,179,236,0.1)" }}>
                      <div className="h-full" style={{ width: "62%", background: "#c4b3ec" }}/>
                    </div>
                    <button onClick={() => setTab("stake")} className="mt-5 ds-btn w-full" style={{ background: "rgba(196,179,236,0.1)", color: "#ebe6f7", border: "1px solid rgba(196,179,236,0.32)" }}>Go to Stake →</button>
                  </div>
                </div>
              </div>
            )}

            {tab === "providers" && (
              <div className="ds-card overflow-x-auto">
                <div className="min-w-[760px]">
                <div className="grid grid-cols-12 gap-4 px-5 py-3 border-b ds-mono text-[10px] uppercase tracking-widest text-[#a89fc7]" style={{ borderColor: "rgba(196,179,236,0.16)" }}>
                  <span className="col-span-3">Provider</span>
                  <span className="col-span-3">Category</span>
                  <span className="col-span-2">Rail</span>
                  <span className="col-span-1">p95</span>
                  <span className="col-span-1">Rep</span>
                  <span className="col-span-2 text-right">Action</span>
                </div>
                {PROVIDERS.map(p => (
                  <div key={p.id} className="grid grid-cols-12 gap-4 px-5 py-4 items-center border-b hover:bg-[rgba(196,179,236,0.04)]" style={{ borderColor: "rgba(196,179,236,0.1)" }}>
                    <div className="col-span-3">
                      <div className="ds-display text-lg">{p.name}</div>
                      <div className="ds-mono text-[11px] text-[#a89fc7]">{p.price}</div>
                    </div>
                    <span className="col-span-3 text-sm text-[#a89fc7]">{p.category}</span>
                    <span className="col-span-2 ds-mono text-xs">{p.rail}</span>
                    <span className="col-span-1 ds-mono text-xs">{p.p95}</span>
                    <span className="col-span-1 ds-mono text-xs text-[#c4b3ec]">{p.rep}</span>
                    <div className="col-span-2 flex justify-end gap-2">
                      <button onClick={() => openNewSession(p.id)} className="text-xs ds-btn" style={{ background: "rgba(196,179,236,0.1)", color: "#ebe6f7", border: "1px solid rgba(196,179,236,0.32)" }}>Open session</button>
                      <button onClick={() => { setSelected(p); setTab("playground"); }} className="text-xs ds-btn" style={{ background: "#c4b3ec", color: "#1a1530" }}>Query →</button>
                    </div>
                  </div>
                ))}
                </div>
              </div>
            )}

            {tab === "playground" && (
              <div className="grid lg:grid-cols-12 gap-5">
                <div className="lg:col-span-4 ds-card p-5">
                  <p className="ds-mono text-[10px] uppercase tracking-widest text-[#a89fc7]">Provider</p>
                  <select value={selected.id} onChange={e => setSelected(PROVIDERS.find(p => p.id === e.target.value)!)} className="ds-input w-full mt-2 px-3 py-2 text-sm">
                    {PROVIDERS.map(p => <option key={p.id} value={p.id}>{p.name} — {p.category}</option>)}
                  </select>
                  <div className="mt-5 space-y-3 text-sm">
                    <div className="flex justify-between"><span className="text-[#a89fc7]">Rail</span><span className="ds-mono text-xs">{selected.rail}</span></div>
                    <div className="flex justify-between"><span className="text-[#a89fc7]">Price</span><span className="ds-mono text-xs">{selected.price}</span></div>
                    <div className="flex justify-between"><span className="text-[#a89fc7]">Reputation</span><span className="ds-mono text-xs text-[#c4b3ec]">{selected.rep}/1000</span></div>
                    <div className="flex justify-between"><span className="text-[#a89fc7]">p95 latency</span><span className="ds-mono text-xs">{selected.p95}</span></div>
                    <div className="flex justify-between"><span className="text-[#a89fc7]">Status</span><span className="ds-mono text-xs">{selected.status}</span></div>
                  </div>
                  <p className="text-xs text-[#a89fc7] leading-relaxed mt-5">{selected.about}</p>
                </div>
                <div className="lg:col-span-8 grid gap-5">
                  <div className="ds-card p-5">
                    <div className="flex items-center justify-between mb-3">
                      <p className="ds-mono text-[10px] uppercase tracking-widest text-[#a89fc7]">Request body</p>
                      <span className="ds-mono text-[10px] text-[#a89fc7]">POST /v1/query/{selected.id}</span>
                    </div>
                    <textarea value={query} onChange={e => setQuery(e.target.value)} rows={7} className="ds-input w-full px-4 py-3 ds-mono text-[13px]"/>
                    <div className="mt-4 flex items-center gap-3 flex-wrap">
                      <button onClick={runQuery} disabled={signing} className="ds-btn" style={{ background: "#c4b3ec", color: "#1a1530" }}>
                        {signing ? "Signing voucher…" : (wallet ? "Sign & send" : "Connect wallet to send")}
                      </button>
                      <span className="ds-mono text-[11px] text-[#a89fc7]">EIP-712 · cumulative auto-incremented</span>
                    </div>
                  </div>
                  <div className="ds-card p-5">
                    <p className="ds-mono text-[10px] uppercase tracking-widest text-[#a89fc7] mb-3">Response</p>
                    <pre className="ds-mono text-[12px] leading-relaxed overflow-x-auto whitespace-pre text-[#ebe6f7]/90 min-h-[140px]">
{response ?? "// awaiting query…"}
                    </pre>
                  </div>
                </div>
              </div>
            )}

            {tab === "sessions" && (
              <div className="ds-card overflow-x-auto">
                <div className="min-w-[680px]">
                <div className="grid grid-cols-12 px-5 py-3 border-b ds-mono text-[10px] uppercase tracking-widest text-[#a89fc7]" style={{ borderColor: "rgba(196,179,236,0.16)" }}>
                  <span className="col-span-3">Session</span><span className="col-span-2">Provider</span><span className="col-span-2">Rail</span><span className="col-span-2">Cumulative</span><span className="col-span-2">Status</span><span className="col-span-1 text-right">Action</span>
                </div>
                {sessions.map(s => (
                  <div key={s.id} className="grid grid-cols-12 px-5 py-4 items-center border-b text-sm" style={{ borderColor: "rgba(196,179,236,0.1)" }}>
                    <span className="col-span-3 ds-mono text-xs text-[#c4b3ec] truncate">{s.id}</span>
                    <span className="col-span-2">{s.prov}</span>
                    <span className="col-span-2 ds-mono text-xs">{s.rail}</span>
                    <span className="col-span-2 ds-mono text-xs">{s.cum.toLocaleString()} PathUSD</span>
                    <span className="col-span-2 ds-mono text-xs">{s.st}</span>
                    <span className="col-span-1 text-right">
                      {s.st === "open" ? (
                        <button onClick={() => closeSession(s.id)} className="text-xs text-[#c4b3ec] hover:underline">close →</button>
                      ) : <span className="text-xs text-[#a89fc7]">—</span>}
                    </span>
                  </div>
                ))}
                </div>
              </div>
            )}

            {tab === "stake" && (
              <div className="grid lg:grid-cols-3 gap-5">
                <div className="ds-card p-6 lg:col-span-2">
                  <p className="ds-mono text-[10px] uppercase tracking-widest text-[#a89fc7]">Underwrite a provider</p>
                  <p className="ds-display text-3xl mt-2">Earn 50% of routing fees</p>
                  <p className="text-sm text-[#a89fc7] mt-3 leading-relaxed">Stake to back a provider's reputation. The other 50% of routing fees is burned — strengthening the network and contracting supply.</p>
                  <div className="mt-6 grid sm:grid-cols-2 gap-4">
                    <label className="block"><span className="ds-mono text-[10px] uppercase tracking-widest text-[#a89fc7]">Provider</span>
                      <select value={stakeProvider} onChange={e => setStakeProvider(e.target.value)} className="ds-input w-full mt-2 px-3 py-2 text-sm">{PROVIDERS.map(p => <option key={p.id} value={p.id}>{p.name}</option>)}</select>
                    </label>
                    <label className="block"><span className="ds-mono text-[10px] uppercase tracking-widest text-[#a89fc7]">Amount</span>
                      <input value={stakeAmount} onChange={e => setStakeAmount(e.target.value)} className="ds-input w-full mt-2 px-3 py-2 text-sm" placeholder="0.00" type="number"/>
                    </label>
                  </div>
                  <button onClick={doStake} className="mt-5 ds-btn" style={{ background: "#c4b3ec", color: "#1a1530" }}>Stake</button>
                </div>
                <div className="ds-card p-6">
                  <p className="ds-mono text-[10px] uppercase tracking-widest text-[#a89fc7]">Your position</p>
                  <p className="ds-display text-4xl mt-2">{staked.toLocaleString()}</p>
                  <p className="text-xs text-[#a89fc7] ds-mono">staked · 4 providers</p>
                  <div className="mt-5 pt-5 border-t" style={{ borderColor: "rgba(196,179,236,0.16)" }}>
                    <p className="text-sm">Claimable rewards</p>
                    <p className="ds-display text-2xl mt-1 text-[#c4b3ec]">+{rewards.toFixed(2)}</p>
                    <button onClick={claimRewards} disabled={rewards <= 0} className="mt-3 text-xs px-3 py-1.5 rounded-full border" style={{ borderColor: "rgba(196,179,236,0.32)", opacity: rewards <= 0 ? 0.5 : 1 }}>Claim</button>
                  </div>
                </div>
              </div>
            )}

            {tab === "reputation" && (
              <div className="grid gap-4">
                {PROVIDERS.map(p => (
                  <div key={p.id} className="ds-card p-5">
                    <div className="flex items-center justify-between mb-3">
                      <div><p className="ds-display text-xl">{p.name}</p><p className="ds-mono text-[11px] text-[#a89fc7]">{p.category}</p></div>
                      <p className="ds-display text-3xl text-[#c4b3ec]">{p.rep}<span className="text-base text-[#a89fc7]">/1000</span></p>
                    </div>
                    <div className="h-2 rounded-full overflow-hidden" style={{ background: "rgba(196,179,236,0.1)" }}>
                      <div className="h-full" style={{ width: (p.rep/10) + "%", background: "#c4b3ec" }}/>
                    </div>
                    <div className="grid grid-cols-3 gap-4 mt-4 ds-mono text-[11px] text-[#a89fc7]">
                      <span>uptime 99.{Math.round(p.rep/100)}%</span><span>p95 {p.p95}</span><span>err 0.{1000-p.rep}%</span>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        </main>
      </div>

      {/* MOBILE NAV DRAWER */}
      <div
        id="ds-mobile-nav"
        className={`lg:hidden fixed inset-0 z-50 transition-opacity duration-300 ${mobileNavOpen ? "opacity-100 visible" : "opacity-0 invisible"}`}
        aria-hidden={!mobileNavOpen}
      >
        <div className="absolute inset-0" style={{ background: "rgba(8,5,18,0.72)", backdropFilter: "blur(8px)" }} onClick={() => setMobileNavOpen(false)}/>
        <aside
          className={`absolute left-0 top-0 bottom-0 w-[82%] max-w-[320px] flex flex-col p-5 border-r transition-transform duration-300 ease-out ${mobileNavOpen ? "translate-x-0" : "-translate-x-full"}`}
          style={{ background: "#15102b", borderColor: "rgba(196,179,236,0.16)" }}
          role="dialog"
          aria-label="Dashboard navigation"
        >
          <div className="flex items-center justify-between mb-8">
            <Link to="/" className="flex items-center gap-2.5" onClick={() => setMobileNavOpen(false)}>
              <img src="/logo.png" alt="" className="w-8 h-8 object-contain"/>
              <span className="ds-display text-[17px]">StargazeMPP</span>
            </Link>
            <button onClick={() => setMobileNavOpen(false)} className="p-2 -mr-2 text-[#a89fc7]" aria-label="Close navigation">
              <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M6 6l12 12M18 6L6 18"/></svg>
            </button>
          </div>
          <nav className="flex flex-col gap-1">
            {NAV.map(n => (
              <button
                key={n.id}
                onClick={() => { setMobileNavOpen(false); n.id === "docs" ? (window.location.href = "/docs") : setTab(n.id); }}
                className={`flex items-center gap-3 px-3 py-3 rounded-lg text-sm transition-colors ${tab === n.id ? "text-[#1a1530]" : "text-[#a89fc7] hover:text-[#ebe6f7]"}`}
                style={tab === n.id ? { background: "#c4b3ec" } : {}}
              >
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d={n.icon}/></svg>
                {n.label}
              </button>
            ))}
          </nav>
          <div className="mt-auto ds-card p-4">
            <p className="ds-mono text-[10px] uppercase tracking-widest text-[#a89fc7]">Wallet</p>
            {wallet ? (
              <>
                <p className="text-sm mt-1 break-all">{shortKey(wallet.publicKey)}</p>
                <p className="ds-mono text-[10px] mt-2 text-[#a89fc7]">{wallet.name} · linked</p>
                <button onClick={() => { disconnect(); setMobileNavOpen(false); }} className="mt-3 text-[11px] ds-mono text-[#c4b3ec] hover:underline">disconnect →</button>
              </>
            ) : (
              <>
                <p className="text-sm mt-1 text-[#a89fc7]">Not connected</p>
                <button onClick={() => { setMobileNavOpen(false); setWalletOpen(true); }} className="mt-3 ds-btn w-full" style={{ background: "#c4b3ec", color: "#1a1530" }}>Connect wallet</button>
              </>
            )}
          </div>
        </aside>
      </div>

      {/* WALLET MODAL */}
      {walletOpen && (
        <div className="ds-modal" onClick={() => setWalletOpen(false)}>
          <div className="ds-card p-7 w-[92%] max-w-[420px]" onClick={e => e.stopPropagation()} style={{ background: "#1f1936" }}>
            <div className="flex items-start justify-between">
              <div>
                <p className="ds-mono text-[10px] uppercase tracking-widest text-[#a89fc7]">Connect wallet</p>
                <h2 className="ds-display text-2xl mt-1">{wallet ? "Connected" : "Choose a wallet"}</h2>
              </div>
              <button onClick={() => setWalletOpen(false)} className="text-[#a89fc7] hover:text-[#ebe6f7] text-xl leading-none">×</button>
            </div>
            {wallet ? (
              <div className="mt-6">
                <div className="flex items-center gap-3 p-4 rounded-xl" style={{ background: "rgba(196,179,236,0.08)" }}>
                  <img src={wallet.name === "Phantom" ? "/phantom.png" : "/solflare.png"} alt="" className="w-10 h-10 rounded-lg object-cover"/>
                  <div className="min-w-0">
                    <p className="text-sm">{wallet.name}</p>
                    <p className="ds-mono text-[11px] text-[#a89fc7] truncate">{wallet.publicKey}</p>
                  </div>
                </div>
                <button onClick={() => { disconnect(); setWalletOpen(false); }} className="mt-5 ds-btn w-full" style={{ background: "rgba(196,179,236,0.1)", color: "#ebe6f7", border: "1px solid rgba(196,179,236,0.32)" }}>Disconnect</button>
              </div>
            ) : (
              <div className="mt-6 space-y-3">
                <button onClick={connectPhantom} disabled={!!walletBusy} className="w-full flex items-center gap-4 p-4 rounded-xl text-left transition-colors hover:bg-[rgba(196,179,236,0.08)]" style={{ border: "1px solid rgba(196,179,236,0.16)" }}>
                  <img src="/phantom.png" alt="" className="w-10 h-10 rounded-lg object-cover"/>
                  <div className="flex-1">
                    <p className="text-sm font-medium">Phantom</p>
                    <p className="ds-mono text-[11px] text-[#a89fc7]">{walletBusy === "Phantom" ? "Connecting…" : "Solana · multi-chain"}</p>
                  </div>
                  <span className="ds-mono text-xs text-[#c4b3ec]">→</span>
                </button>
                <button onClick={connectSolflare} disabled={!!walletBusy} className="w-full flex items-center gap-4 p-4 rounded-xl text-left transition-colors hover:bg-[rgba(196,179,236,0.08)]" style={{ border: "1px solid rgba(196,179,236,0.16)" }}>
                  <img src="/solflare.png" alt="" className="w-10 h-10 rounded-lg object-cover"/>
                  <div className="flex-1">
                    <p className="text-sm font-medium">Solflare</p>
                    <p className="ds-mono text-[11px] text-[#a89fc7]">{walletBusy === "Solflare" ? "Connecting…" : "Solana native"}</p>
                  </div>
                  <span className="ds-mono text-xs text-[#c4b3ec]">→</span>
                </button>
                {walletErr && <p className="text-xs text-[#ff9eb1] mt-2">{walletErr}</p>}
                <p className="ds-mono text-[10px] uppercase tracking-widest text-[#a89fc7] pt-3 text-center">Browser extension required</p>
              </div>
            )}
          </div>
        </div>
      )}

      {toast && <div className="ds-toast">{toast}</div>}
    </div>
  );
}

export default Dashboard;
