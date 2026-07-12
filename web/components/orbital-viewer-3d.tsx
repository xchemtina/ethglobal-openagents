"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import { cn } from "@/lib/utils";

/** [x, y, z, sign, magnitude?] from cube sample export */
type Pt = number[];

interface OrbitalFile {
  molecule_label: string;
  result_artifact_id: string;
  method?: string;
  energy_hartree?: number;
  gap_ev?: number;
  atoms: Array<{
    element: string;
    position: [number, number, number];
  }>;
  bonds: Array<{
    start: [number, number, number];
    end: [number, number, number];
  }>;
  orbitals: Array<{ label: string; points: Pt[] }>;
}

const CPK: Record<string, string> = {
  H: "#f0f0f0",
  C: "#909090",
  N: "#3050f8",
  O: "#ff0d0d",
  Ge: "#668f8f",
  Sn: "#668080",
  default: "#a0ffa0",
};

const ATOM_R: Record<string, number> = {
  H: 0.32,
  C: 0.55,
  N: 0.52,
  O: 0.5,
  default: 0.5,
};

/**
 * Interactive 3D orbital field (canvas).
 * Auto-rotates, depth-sorts soft isosurface points, normalizes molecule scale.
 */
export function OrbitalViewer3D({
  dataUrl,
  className,
}: {
  dataUrl: string;
  className?: string;
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [data, setData] = useState<OrbitalFile | null>(null);
  const [orbIdx, setOrbIdx] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [autoSpin, setAutoSpin] = useState(true);
  const rot = useRef({ yaw: 0.85, pitch: 0.42 });
  const zoom = useRef(1);
  const drag = useRef<{ x: number; y: number } | null>(null);
  const spinPausedUntil = useRef(0);

  useEffect(() => {
    let cancelled = false;
    setError(null);
    setData(null);
    fetch(dataUrl)
      .then((r) => {
        if (!r.ok) throw new Error(`HTTP ${r.status}`);
        return r.json();
      })
      .then((j) => {
        if (!cancelled) {
          setData(j as OrbitalFile);
          setOrbIdx(0);
          zoom.current = 1;
          rot.current = { yaw: 0.85, pitch: 0.42 };
        }
      })
      .catch((e) => {
        if (!cancelled) setError(e instanceof Error ? e.message : String(e));
      });
    return () => {
      cancelled = true;
    };
  }, [dataUrl]);

  const frame = useMemo(() => {
    if (!data) return null;
    let minX = Infinity,
      minY = Infinity,
      minZ = Infinity;
    let maxX = -Infinity,
      maxY = -Infinity,
      maxZ = -Infinity;
    const consider = (x: number, y: number, z: number) => {
      minX = Math.min(minX, x);
      minY = Math.min(minY, y);
      minZ = Math.min(minZ, z);
      maxX = Math.max(maxX, x);
      maxY = Math.max(maxY, y);
      maxZ = Math.max(maxZ, z);
    };
    for (const a of data.atoms || []) {
      consider(a.position[0], a.position[1], a.position[2]);
    }
    for (const o of data.orbitals || []) {
      for (const p of o.points || []) consider(p[0], p[1], p[2]);
    }
    if (!Number.isFinite(minX)) {
      minX = minY = minZ = -1;
      maxX = maxY = maxZ = 1;
    }
    const cx = (minX + maxX) / 2;
    const cy = (minY + maxY) / 2;
    const cz = (minZ + maxZ) / 2;
    const span = Math.max(maxX - minX, maxY - minY, maxZ - minZ, 0.4);
    return { cx, cy, cz, span };
  }, [data]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !data || !frame) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    let raf = 0;
    const dpr = Math.min(window.devicePixelRatio || 1, 2);

    const resize = () => {
      const rect = canvas.getBoundingClientRect();
      canvas.width = Math.floor(rect.width * dpr);
      canvas.height = Math.floor(rect.height * dpr);
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    };
    resize();
    window.addEventListener("resize", resize);

    const project = (x: number, y: number, z: number, w: number, h: number) => {
      // center + normalize
      let X = (x - frame.cx) / frame.span;
      let Y = (y - frame.cy) / frame.span;
      let Z = (z - frame.cz) / frame.span;

      const cy = Math.cos(rot.current.yaw);
      const sy = Math.sin(rot.current.yaw);
      const cp = Math.cos(rot.current.pitch);
      const sp = Math.sin(rot.current.pitch);

      // yaw then pitch
      let x1 = X * cy - Z * sy;
      let z1 = X * sy + Z * cy;
      let y1 = Y * cp - z1 * sp;
      z1 = Y * sp + z1 * cp;

      const zCam = 2.6;
      const persp = zCam / (zCam + z1);
      const scale = Math.min(w, h) * 0.9 * zoom.current;
      return {
        X: w / 2 + x1 * scale * persp,
        Y: h / 2 - y1 * scale * persp,
        Z: z1,
        s: persp,
      };
    };

    const softBlob = (
      x: number,
      y: number,
      r: number,
      rgb: [number, number, number],
      alpha: number,
    ) => {
      const [cr, cg, cb] = rgb;
      const g = ctx.createRadialGradient(x, y, 0, x, y, r);
      g.addColorStop(0, `rgba(${cr},${cg},${cb},${alpha})`);
      g.addColorStop(0.4, `rgba(${cr},${cg},${cb},${alpha * 0.35})`);
      g.addColorStop(1, `rgba(${cr},${cg},${cb},0)`);
      ctx.beginPath();
      ctx.fillStyle = g;
      ctx.arc(x, y, r, 0, Math.PI * 2);
      ctx.fill();
    };

    const draw = () => {
      const now = performance.now();
      if (autoSpin && now > spinPausedUntil.current && !drag.current) {
        rot.current.yaw += 0.008;
      }

      const rect = canvas.getBoundingClientRect();
      const w = rect.width;
      const h = rect.height;

      // deep space background
      const bg = ctx.createRadialGradient(w * 0.5, h * 0.45, 0, w * 0.5, h * 0.5, Math.max(w, h) * 0.7);
      bg.addColorStop(0, "#0a1220");
      bg.addColorStop(1, "#03060c");
      ctx.fillStyle = bg;
      ctx.fillRect(0, 0, w, h);

      // ground plane ellipse for 3D cue
      ctx.save();
      ctx.translate(w / 2, h * 0.72);
      ctx.scale(1, 0.28);
      ctx.beginPath();
      ctx.ellipse(0, 0, Math.min(w, h) * 0.38 * zoom.current, Math.min(w, h) * 0.38 * zoom.current, 0, 0, Math.PI * 2);
      ctx.strokeStyle = "rgba(80,140,220,0.12)";
      ctx.lineWidth = 1.5;
      ctx.stroke();
      ctx.restore();

      const orb = data.orbitals[orbIdx];
      const pts = orb?.points ?? [];

      const projected = pts.map((p) => {
        const pr = project(p[0], p[1], p[2], w, h);
        const mag = p.length > 4 ? Math.abs(p[4]) : 1;
        return { ...pr, sign: p[3] ?? 1, mag };
      });
      projected.sort((a, b) => a.Z - b.Z);

      // orbital field as soft volumetric blobs
      for (const p of projected) {
        const depthFade = Math.min(1, 0.35 + 0.75 * p.s);
        const r = (3.5 + 10 * p.s * Math.min(1.4, 0.35 + p.mag)) * (0.85 + 0.3 * p.mag);
        const a = Math.min(0.72, 0.08 + 0.55 * depthFade * Math.min(1, p.mag + 0.15));
        if (p.sign >= 0) {
          softBlob(p.X, p.Y, r, [56, 220, 255], a);
        } else {
          softBlob(p.X, p.Y, r, [255, 170, 50], a);
        }
      }

      // bonds (depth-sorted)
      type BondDraw = { a: ReturnType<typeof project>; c: ReturnType<typeof project>; z: number };
      const bonds: BondDraw[] = (data.bonds || []).map((b) => {
        const a = project(b.start[0], b.start[1], b.start[2], w, h);
        const c = project(b.end[0], b.end[1], b.end[2], w, h);
        return { a, c, z: (a.Z + c.Z) / 2 };
      });
      bonds.sort((u, v) => u.z - v.z);
      for (const b of bonds) {
        ctx.beginPath();
        ctx.strokeStyle = `rgba(200,220,255,${0.25 + 0.35 * ((b.a.s + b.c.s) / 2)})`;
        ctx.lineWidth = 1.2 + 2.2 * ((b.a.s + b.c.s) / 2);
        ctx.lineCap = "round";
        ctx.moveTo(b.a.X, b.a.Y);
        ctx.lineTo(b.c.X, b.c.Y);
        ctx.stroke();
      }

      // atoms
      const atoms = (data.atoms || []).map((atom) => {
        const p = project(atom.position[0], atom.position[1], atom.position[2], w, h);
        return { atom, p };
      });
      atoms.sort((a, b) => a.p.Z - b.p.Z);
      for (const { atom, p } of atoms) {
        const baseR = ATOM_R[atom.element] || ATOM_R.default;
        const r = (4 + 10 * baseR) * p.s;
        const col = CPK[atom.element] || CPK.default;
        // sphere shading
        const g = ctx.createRadialGradient(
          p.X - r * 0.3,
          p.Y - r * 0.35,
          r * 0.1,
          p.X,
          p.Y,
          r,
        );
        g.addColorStop(0, "#ffffff");
        g.addColorStop(0.15, col);
        g.addColorStop(1, shade(col, 0.35));
        ctx.beginPath();
        ctx.fillStyle = g;
        ctx.arc(p.X, p.Y, r, 0, Math.PI * 2);
        ctx.fill();
        ctx.strokeStyle = "rgba(0,0,0,0.4)";
        ctx.lineWidth = 1;
        ctx.stroke();
      }

      // HUD
      ctx.fillStyle = "rgba(180,200,220,0.75)";
      ctx.font = "11px ui-monospace, SFMono-Regular, monospace";
      ctx.fillText(
        "drag · scroll zoom · cyan + / amber − phase",
        12,
        h - 14,
      );
      ctx.fillStyle = "rgba(120,180,255,0.55)";
      ctx.fillText(autoSpin ? "auto-spin on" : "auto-spin off", w - 100, h - 14);

      raf = requestAnimationFrame(draw);
    };
    draw();

    const onDown = (e: PointerEvent) => {
      drag.current = { x: e.clientX, y: e.clientY };
      spinPausedUntil.current = performance.now() + 2500;
      canvas.setPointerCapture(e.pointerId);
    };
    const onMove = (e: PointerEvent) => {
      if (!drag.current) return;
      const dx = e.clientX - drag.current.x;
      const dy = e.clientY - drag.current.y;
      drag.current = { x: e.clientX, y: e.clientY };
      rot.current.yaw += dx * 0.01;
      rot.current.pitch += dy * 0.01;
      rot.current.pitch = Math.max(-1.35, Math.min(1.35, rot.current.pitch));
    };
    const onUp = () => {
      drag.current = null;
    };
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      zoom.current *= e.deltaY > 0 ? 0.92 : 1.08;
      zoom.current = Math.max(0.45, Math.min(2.8, zoom.current));
      spinPausedUntil.current = performance.now() + 1500;
    };
    canvas.addEventListener("pointerdown", onDown);
    canvas.addEventListener("pointermove", onMove);
    canvas.addEventListener("pointerup", onUp);
    canvas.addEventListener("pointercancel", onUp);
    canvas.addEventListener("wheel", onWheel, { passive: false });

    return () => {
      cancelAnimationFrame(raf);
      window.removeEventListener("resize", resize);
      canvas.removeEventListener("pointerdown", onDown);
      canvas.removeEventListener("pointermove", onMove);
      canvas.removeEventListener("pointerup", onUp);
      canvas.removeEventListener("pointercancel", onUp);
      canvas.removeEventListener("wheel", onWheel);
    };
  }, [data, orbIdx, frame, autoSpin]);

  if (error) {
    return (
      <div
        className={cn(
          "flex aspect-square min-h-[320px] items-center justify-center rounded-xl border border-border bg-black/50 p-8 text-sm text-red-400",
          className,
        )}
      >
        3D load failed: {error}
      </div>
    );
  }
  if (!data) {
    return (
      <div
        className={cn(
          "flex aspect-square min-h-[320px] items-center justify-center rounded-xl border border-border bg-black/50 p-8 text-sm text-muted-foreground",
          className,
        )}
      >
        Loading orbital field…
      </div>
    );
  }

  return (
    <div
      className={cn(
        "flex flex-col overflow-hidden rounded-xl border border-border bg-black/40",
        className,
      )}
    >
      <canvas
        ref={canvasRef}
        className="aspect-square w-full cursor-grab touch-none active:cursor-grabbing"
        style={{ minHeight: 360 }}
      />
      <div className="flex flex-wrap items-center gap-2 border-t border-border bg-card/40 p-3">
        {data.orbitals.map((o, i) => (
          <button
            key={o.label}
            type="button"
            onClick={() => setOrbIdx(i)}
            className={cn(
              "rounded-md px-3 py-1.5 font-mono text-xs",
              orbIdx === i
                ? "bg-primary text-primary-foreground"
                : "bg-muted text-muted-foreground hover:text-foreground",
            )}
          >
            {o.label}
          </button>
        ))}
        <button
          type="button"
          onClick={() => setAutoSpin((v) => !v)}
          className={cn(
            "rounded-md px-3 py-1.5 font-mono text-xs",
            autoSpin
              ? "bg-emerald-500/20 text-emerald-300"
              : "bg-muted text-muted-foreground",
          )}
        >
          {autoSpin ? "spin · on" : "spin · off"}
        </button>
        <span className="ml-auto font-mono text-[10px] text-muted-foreground">
          interactive 3D · cube sample field
        </span>
      </div>
    </div>
  );
}

function shade(hex: string, factor: number): string {
  const m = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
  if (!m) return hex;
  const r = Math.round(parseInt(m[1], 16) * factor);
  const g = Math.round(parseInt(m[2], 16) * factor);
  const b = Math.round(parseInt(m[3], 16) * factor);
  return `rgb(${r},${g},${b})`;
}
