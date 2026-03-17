import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  ReactFlow,
  Background,
  BackgroundVariant,
  Controls,
  MiniMap,
  Handle,
  Panel,
  Position,
  useEdgesState,
  addEdge,
  applyNodeChanges,
  type Connection,
  type Edge,
  type IsValidConnection,
  type Node,
  type NodeChange,
  type NodeProps,
  type NodeTypes,
  type ReactFlowInstance,
} from "@xyflow/react";
import type { CommandEntry, DefinitionEntry, HookState, WsResult } from "../types";
import { t } from "../i18n";

type SendFn = (method: string, params?: Record<string, unknown>) => Promise<WsResult>;

const NODE_W = 160;
const NODE_H = 40;
const H_GAP = 60;
const V_GAP = 14;
const HDR_H = 36;
const SEC_PAD = 16;
const SEC_GAP = 28;
const EDGE_STYLE = { stroke: "#00d4aa55", strokeWidth: 1.5 };
const NEW_W = 204;
const NEW_H = 144;

// ── Custom node: hook group container ─────────────────────────────────────────

function HookGroupNode({ data }: NodeProps) {
  return (
    <div
      style={{
        width: "100%",
        height: "100%",
        background: "rgba(0, 212, 170, 0.025)",
        border: "1px solid rgba(0, 212, 170, 0.25)",
        borderRadius: 10,
        boxSizing: "border-box",
        position: "relative",
      }}
    >
      <span
        style={{
          position: "absolute",
          top: 10,
          left: 14,
          fontSize: 11,
          fontWeight: 600,
          color: "rgba(0, 212, 170, 0.6)",
          fontFamily: '"SF Mono","Fira Code",monospace',
          letterSpacing: 1,
          pointerEvents: "none",
          userSelect: "none",
        }}
      >
        {data.label as string}
      </span>
    </div>
  );
}

// ── Custom node: command (with handles for dependency edges) ───────────────────

type CmdData = {
  label: string;
  run: string;
  isRef: boolean;
  isInclude?: boolean;
  includeRef?: string;
  includePath?: string;
  includeArgs?: string;
  refName?: string;
  refArgs?: string;
  nameOverride?: string;
  hasOverrides: boolean;
  test: boolean;
  selected: boolean;
};

function cmdColor(d: CmdData) {
  if (d.isInclude) return "#f97316"; // orange for include refs
  if (d.hasOverrides) return "#c084fc";
  if (d.isRef) return "#6496ff";
  if (d.test) return "#ff8844";
  return "#00d4aa";
}

function CmdNodeComponent({ data }: NodeProps) {
  const d = data as CmdData;
  const color = cmdColor(d);
  const border = `${d.selected ? "2px" : "1.5px"} solid ${color}${d.selected ? "cc" : "55"}`;
  const bg = `${color}${d.selected ? "18" : "0d"}`;
  const handleStyle = { background: color, width: 8, height: 8, border: `1px solid ${color}` };

  return (
    <>
      <Handle type="target" position={Position.Left} style={handleStyle} />
      <div
        style={{
          width: "100%",
          height: "100%",
          background: bg,
          border,
          borderRadius: 7,
          color,
          fontFamily: '"SF Mono","Fira Code",monospace',
          fontSize: 11.5,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          boxSizing: "border-box",
          userSelect: "none",
          padding: "0 6px",
          textAlign: "center",
          lineHeight: 1.3,
          cursor: "pointer",
          transition: "background 0.12s, border 0.12s",
        }}
      >
        {d.label}
      </div>
      <Handle type="source" position={Position.Right} style={handleStyle} />
    </>
  );
}

// ── Custom node: new command form ──────────────────────────────────────────────

type NewCmdData = {
  hookName: string;
  pendingEdgeFrom?: string;
  onSave: (name: string, run: string, testOnly: boolean) => void;
  onCancel: () => void;
};

function NewCmdNode({ data }: NodeProps) {
  const d = data as NewCmdData;
  const [name, setName] = useState("");
  const [run, setRun] = useState("");
  const [testOnly, setTestOnly] = useState(false);

  const inputStyle: React.CSSProperties = {
    background: "rgba(255,255,255,0.05)",
    border: "1px solid rgba(0,212,170,0.25)",
    borderRadius: 4,
    padding: "3px 7px",
    color: "#e6edf3",
    fontSize: 11,
    outline: "none",
    fontFamily: '"SF Mono","Fira Code",monospace',
    width: "100%",
    boxSizing: "border-box",
  };

  const handleStyle = { background: "#00d4aa", width: 8, height: 8, border: "1px solid #00d4aa" };

  return (
    <>
      <Handle type="target" position={Position.Left} style={handleStyle} />
      <div
        style={{
          width: "100%",
          height: "100%",
          background: "rgba(13,17,23,0.92)",
          border: "1.5px dashed rgba(0,212,170,0.55)",
          borderRadius: 8,
          padding: "9px 11px",
          boxSizing: "border-box",
          display: "flex",
          flexDirection: "column",
          gap: 5,
          fontFamily: '"SF Mono","Fira Code",monospace',
          fontSize: 11,
          color: "#00d4aa",
          backdropFilter: "blur(6px)",
        }}
      >
        <input
          className="nodrag nopan"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder={t("name")}
          style={inputStyle}
          // eslint-disable-next-line jsx-a11y/no-autofocus
          autoFocus
        />
        <input
          className="nodrag nopan"
          value={run}
          onChange={(e) => setRun(e.target.value)}
          placeholder={t("run")}
          style={inputStyle}
        />
        <label
          className="nodrag nopan"
          style={{
            display: "flex",
            alignItems: "center",
            gap: 5,
            cursor: "pointer",
            userSelect: "none",
            fontSize: 10,
            color: "#ff8844",
          }}
        >
          <input
            type="checkbox"
            className="nodrag nopan"
            checked={testOnly}
            onChange={(e) => setTestOnly(e.target.checked)}
          />
          {t("test_only")}
        </label>
        <div className="nodrag nopan" style={{ display: "flex", gap: 5, marginTop: 1 }}>
          <button
            className="nodrag nopan"
            disabled={!name.trim() || !run.trim()}
            onClick={() => d.onSave(name.trim(), run.trim(), testOnly)}
            style={{
              flex: 1,
              background: "rgba(0,212,170,0.15)",
              border: "1px solid rgba(0,212,170,0.4)",
              borderRadius: 4,
              color: "#00d4aa",
              fontSize: 10,
              padding: "3px 0",
              cursor: "pointer",
              fontFamily: "inherit",
            }}
          >
            {t("save")}
          </button>
          <button
            className="nodrag nopan"
            onClick={() => d.onCancel()}
            style={{
              flex: 1,
              background: "transparent",
              border: "1px solid rgba(255,255,255,0.15)",
              borderRadius: 4,
              color: "rgba(255,255,255,0.35)",
              fontSize: 10,
              padding: "3px 0",
              cursor: "pointer",
              fontFamily: "inherit",
            }}
          >
            {t("cancel")}
          </button>
        </div>
      </div>
      <Handle type="source" position={Position.Right} style={handleStyle} />
    </>
  );
}

// Module-level constant — must not be defined inside the component
const nodeTypes: NodeTypes = {
  hookGroup: HookGroupNode,
  cmdNode: CmdNodeComponent,
  newCmdNode: NewCmdNode,
};

// ── Palette panel ──────────────────────────────────────────────────────────────

function FlowPalette({ definitions }: { definitions: DefinitionEntry[] }) {
  const onDragStart = (e: React.DragEvent, type: string, payload = "") => {
    e.dataTransfer.setData("application/githops-type", type);
    e.dataTransfer.setData("application/githops-payload", payload);
    e.dataTransfer.effectAllowed = "copy";
  };

  const chipBase: React.CSSProperties = {
    fontSize: 10,
    fontFamily: '"SF Mono","Fira Code",monospace',
    padding: "4px 8px",
    borderRadius: 5,
    cursor: "grab",
    userSelect: "none",
    whiteSpace: "nowrap",
  };

  return (
    <Panel position="top-left">
      <div
        style={{
          background: "rgba(13,17,23,0.88)",
          backdropFilter: "blur(8px)",
          border: "1px solid rgba(0,212,170,0.2)",
          borderRadius: 8,
          padding: "8px",
          display: "flex",
          flexDirection: "column",
          gap: 4,
          minWidth: 120,
        }}
      >
        <span
          style={{
            fontSize: 9,
            color: "rgba(0,212,170,0.45)",
            fontFamily: '"SF Mono",monospace',
            letterSpacing: 1,
            paddingBottom: 4,
            borderBottom: "1px solid rgba(0,212,170,0.12)",
          }}
        >
          {t("palette_title")}
        </span>
        <div
          draggable
          onDragStart={(e) => onDragStart(e, "command")}
          style={{
            ...chipBase,
            background: "rgba(0,212,170,0.08)",
            border: "1px solid rgba(0,212,170,0.3)",
            color: "#00d4aa",
          }}
        >
          {t("add_command")}
        </div>
        {definitions.length > 0 && (
          <>
            <span
              style={{
                fontSize: 9,
                color: "rgba(100,150,255,0.45)",
                fontFamily: '"SF Mono",monospace',
                letterSpacing: 1,
                paddingTop: 2,
              }}
            >
              {t("definitions")}
            </span>
            {definitions.map((d) => (
              <div
                key={d.name}
                draggable
                onDragStart={(e) => onDragStart(e, "definition", d.name)}
                style={{
                  ...chipBase,
                  background: "rgba(100,150,255,0.08)",
                  border: "1px solid rgba(100,150,255,0.3)",
                  color: "#6496ff",
                }}
              >
                {d.name}
              </div>
            ))}
          </>
        )}
      </div>
    </Panel>
  );
}

// ── Graph layout builder ───────────────────────────────────────────────────────

type ResolvedCmd = {
  name: string;
  run: string;
  test: boolean;
  isRef: boolean;
  isInclude?: boolean;
  includeRef?: string;
  includePath?: string;
  includeArgs?: string;
  refName?: string;
  refArgs?: string;
  nameOverride?: string;
  hasOverrides: boolean;
};

function resolveCommands(hook: HookState, defs: DefinitionEntry[]): ResolvedCmd[] {
  const result: ResolvedCmd[] = [];
  for (const entry of hook.commands) {
    if (entry.isRef) {
      const def = defs.find((d) => d.name === entry.refName);
      const hasOverrides = !!(entry.refArgs || entry.nameOverride);
      if (def) {
        const cmds = def.type === "single" ? [def.commands[0]] : def.commands;
        cmds.forEach((c) => {
          const name = entry.nameOverride || c.name;
          const run = entry.refArgs ? `${c.run} ${entry.refArgs}` : c.run;
          result.push({
            name, run, test: c.test, isRef: true,
            refName: entry.refName, refArgs: entry.refArgs,
            nameOverride: entry.nameOverride, hasOverrides,
          });
        });
      } else {
        result.push({
          name: entry.nameOverride || entry.refName, run: "", test: false, isRef: true,
          refName: entry.refName, refArgs: entry.refArgs,
          nameOverride: entry.nameOverride, hasOverrides,
        });
      }
    } else if (entry.isInclude) {
      result.push({
        name: entry.name,
        run: entry.run,
        test: false,
        isRef: false,
        isInclude: true,
        includeRef: entry.includeRef,
        includePath: entry.includePath,
        includeArgs: entry.args,
        hasOverrides: false,
      });
    } else {
      result.push({ name: entry.name, run: entry.run, test: entry.test, isRef: false, hasOverrides: false });
    }
  }
  return result;
}

function computeLayers(cmds: Array<ResolvedCmd & { depends: string[] }>): number[] {
  const idx = new Map(cmds.map((c, i) => [c.name, i]));
  const layers = new Array<number>(cmds.length).fill(0);
  const vis = new Set<number>();
  function layer(i: number): number {
    if (vis.has(i)) return layers[i];
    vis.add(i);
    let max = -1;
    for (const dep of cmds[i].depends) {
      const j = idx.get(dep);
      if (j !== undefined) max = Math.max(max, layer(j));
    }
    layers[i] = max + 1;
    return layers[i];
  }
  cmds.forEach((_, i) => layer(i));
  return layers;
}

function buildGraph(
  hooks: HookState[],
  defs: DefinitionEntry[],
): { nodes: Node[]; edges: Edge[] } {
  const nodes: Node[] = [];
  const edges: Edge[] = [];
  let curY = SEC_PAD;

  for (const hook of hooks) {
    if (!hook.configured || !hook.enabled || !hook.commands.length) continue;
    const cmds = resolveCommands(hook, defs);
    if (!cmds.length) continue;

    const hookCmds = hook.commands;
    const withDeps = cmds.map((c, i) => ({
      ...c,
      depends: hookCmds[i]?.depends ?? [],
    }));
    const layers = computeLayers(withDeps);
    const maxLayer = Math.max(...layers, 0);

    const byLayer: number[][] = Array.from({ length: maxLayer + 1 }, () => []);
    cmds.forEach((_, i) => byLayer[layers[i]].push(i));
    const maxPerLayer = Math.max(...byLayer.map((l) => l.length), 1);
    const contentH = maxPerLayer * (NODE_H + V_GAP) - V_GAP;
    const groupW = (maxLayer + 1) * (NODE_W + H_GAP) - H_GAP + 2 * SEC_PAD;
    const groupH = HDR_H + 2 * SEC_PAD + contentH;
    const groupId = `${hook.name}::__group`;

    nodes.push({
      id: groupId,
      type: "hookGroup",
      position: { x: 0, y: curY },
      data: { label: hook.name },
      style: { width: groupW, height: groupH },
      draggable: false,
      selectable: false,
      connectable: false,
    });

    for (let li = 0; li <= maxLayer; li++) {
      const col = byLayer[li];
      const totalH = col.length * (NODE_H + V_GAP) - V_GAP;
      col.forEach((cidx, ni) => {
        const cmd = cmds[cidx];
        const nodeId = `${hook.name}::${cmd.name}`;
        nodes.push({
          id: nodeId,
          type: "cmdNode",
          parentId: groupId,
          extent: "parent",
          position: {
            x: SEC_PAD + li * (NODE_W + H_GAP),
            y: HDR_H + SEC_PAD + (contentH - totalH) / 2 + ni * (NODE_H + V_GAP),
          },
          data: {
            label: cmd.name,
            run: cmd.run,
            isRef: cmd.isRef,
            isInclude: cmd.isInclude,
            includeRef: cmd.includeRef,
            includePath: cmd.includePath,
            includeArgs: cmd.includeArgs,
            refName: cmd.refName,
            refArgs: cmd.refArgs,
            nameOverride: cmd.nameOverride,
            hasOverrides: cmd.hasOverrides,
            test: cmd.test,
            selected: false,
          },
          style: { width: NODE_W, height: NODE_H },
          draggable: false,
          selectable: false,
        });
        for (const dep of hookCmds[cidx]?.depends ?? []) {
          const srcId = `${hook.name}::${dep}`;
          edges.push({
            id: `${srcId}->${nodeId}`,
            source: srcId,
            target: nodeId,
            style: EDGE_STYLE,
          });
        }
      });
    }

    curY += groupH + SEC_GAP;
  }

  return { nodes, edges };
}

// ── Node detail panel ──────────────────────────────────────────────────────────

function NodeDetailPanel({ data, onClose }: { data: CmdData; onClose: () => void }) {
  const color = cmdColor(data);
  const previewRun = data.isRef && data.refArgs && data.run
    ? data.run  // run is already the resolved command (defRun + args) from resolveCommands
    : data.run;
  const defRun = data.isRef && data.refArgs && data.run
    ? data.run.slice(0, data.run.length - data.refArgs.length - 1)
    : null;

  return (
    <Panel position="bottom-right">
      <div
        style={{
          background: "rgba(13,17,23,0.95)",
          backdropFilter: "blur(10px)",
          border: `1px solid ${color}55`,
          borderRadius: 10,
          padding: "12px 14px",
          minWidth: 240,
          maxWidth: 320,
          fontFamily: '"SF Mono","Fira Code",monospace',
          fontSize: 11,
          color: "rgba(255,255,255,0.75)",
          display: "flex",
          flexDirection: "column",
          gap: 8,
        }}
      >
        {/* Header */}
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
          <span style={{ color, fontWeight: 600, fontSize: 13 }}>{data.label}</span>
          <button
            onClick={onClose}
            style={{
              background: "transparent",
              border: "none",
              color: "rgba(255,255,255,0.35)",
              cursor: "pointer",
              fontSize: 14,
              lineHeight: 1,
              padding: "0 2px",
            }}
          >
            ×
          </button>
        </div>

        {/* Fields */}
        <div style={{ display: "flex", flexDirection: "column", gap: 5 }}>
          {data.isRef && data.refName && (
            <Row label={t("ref_label")} value={data.refName} color="rgba(100,150,255,0.9)" />
          )}
          {data.isInclude && data.includeRef && (
            <Row label={t("include_ref_label")} value={data.includeRef} color="rgba(249,115,22,0.9)" />
          )}
          {data.isInclude && data.includePath && (
            <Row label={t("include_run")} value={data.includePath} />
          )}
          {data.isInclude && data.includeArgs && (
            <Row label={t("include_args")} value={data.includeArgs} />
          )}
          {data.isRef && data.refArgs ? (
            <>
              {defRun && <Row label={t("run")} value={defRun} />}
              <Row label={t("ref_args")} value={data.refArgs} />
              <Row label={t("preview")} value={previewRun} color={color} mono />
            </>
          ) : (
            data.run && <Row label={t("run")} value={previewRun} mono />
          )}
          {data.test && (
            <div style={{ display: "flex", alignItems: "center", gap: 5 }}>
              <span style={{ fontSize: 9, color: "#ff884488", textTransform: "uppercase", letterSpacing: 1 }}>
                {t("test_only")}
              </span>
            </div>
          )}
        </div>
      </div>
    </Panel>
  );
}

function Row({
  label, value, color = "rgba(255,255,255,0.55)", mono = false,
}: {
  label: string; value: string; color?: string; mono?: boolean;
}) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 1 }}>
      <span style={{ fontSize: 9, color: "rgba(255,255,255,0.3)", textTransform: "uppercase", letterSpacing: 1 }}>
        {label}
      </span>
      <span
        style={{
          color,
          fontSize: 11,
          wordBreak: "break-all",
          fontFamily: mono ? '"SF Mono","Fira Code",monospace' : "inherit",
        }}
      >
        {value}
      </span>
    </div>
  );
}

// ── FlowGraph component ────────────────────────────────────────────────────────

type Props = {
  hooks: HookState[];
  definitions: DefinitionEntry[];
  send: SendFn;
};

export function FlowGraph({ hooks, definitions, send }: Props) {
  const { nodes: computedNodes, edges: computedEdges } = useMemo(
    () => buildGraph(hooks, definitions),
    [hooks, definitions],
  );

  const [localEdges, setLocalEdges, onEdgesChange] = useEdgesState(computedEdges);
  const [extraNodes, setExtraNodes] = useState<Node[]>([]);
  const [rfInstance, setRfInstance] = useState<ReactFlowInstance | null>(null);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);

  // Always-fresh reference to hooks so callbacks don't stale-close over an old value
  const hooksRef = useRef(hooks);
  hooksRef.current = hooks;

  useEffect(() => {
    setLocalEdges(computedEdges);
  }, [computedEdges, setLocalEdges]);

  const allNodes = useMemo(
    () => [
      ...computedNodes.map((n) =>
        n.type === "cmdNode"
          ? { ...n, data: { ...n.data, selected: n.id === selectedNodeId } }
          : n,
      ),
      ...extraNodes,
    ],
    [computedNodes, extraNodes, selectedNodeId],
  );

  const selectedNodeData = useMemo(() => {
    const node = computedNodes.find((n) => n.id === selectedNodeId);
    return node?.type === "cmdNode" ? (node.data as CmdData) : null;
  }, [computedNodes, selectedNodeId]);

  const onNodeClick = useCallback((_: React.MouseEvent, node: Node) => {
    if (node.type !== "cmdNode") return;
    setSelectedNodeId((prev) => (prev === node.id ? null : node.id));
  }, []);

  const onPaneClick = useCallback(() => {
    setSelectedNodeId(null);
  }, []);

  // Forward position changes only for draggable extra nodes
  const onNodesChange = useCallback((changes: NodeChange[]) => {
    setExtraNodes((prev) => {
      const extraIds = new Set(prev.map((n) => n.id));
      const relevant = changes.filter(
        (c) => "id" in c && extraIds.has((c as { id: string }).id),
      );
      return relevant.length ? applyNodeChanges(relevant, prev) : prev;
    });
  }, []);

  // Create a new-command form node at `pos`, optionally pre-wired from `pendingEdgeFrom`
  const addNewCmdNode = useCallback(
    (pos: { x: number; y: number }, hookName: string, pendingEdgeFrom?: string) => {
      const id = `__new__${Date.now()}`;
      const pendingEdgeId = pendingEdgeFrom
        ? `${pendingEdgeFrom}->pending-${id}`
        : undefined;

      const onSave = (name: string, run: string, testOnly: boolean) => {
        setExtraNodes((prev) => prev.filter((n) => n.id !== id));
        if (pendingEdgeId)
          setLocalEdges((prev) => prev.filter((e) => e.id !== pendingEdgeId));

        const hook = hooksRef.current.find((h) => h.name === hookName);
        if (!hook) return;

        const depends: string[] = [];
        if (pendingEdgeFrom) {
          const [, srcCmd] = pendingEdgeFrom.split("::");
          if (srcCmd && srcCmd !== "__group") depends.push(srcCmd);
        }

        const newCmd: CommandEntry = {
          name,
          run,
          test: testOnly,
          isRef: false,
          refName: "",
          depends,
          env: {},
        };
        send("hook.update", {
          hook: hookName,
          enabled: hook.enabled,
          parallel: hook.parallel,
          commands: [...hook.commands, newCmd],
        });
      };

      const onCancel = () => {
        setExtraNodes((prev) => prev.filter((n) => n.id !== id));
        if (pendingEdgeId)
          setLocalEdges((prev) => prev.filter((e) => e.id !== pendingEdgeId));
      };

      setExtraNodes((prev) => [
        ...prev,
        {
          id,
          type: "newCmdNode",
          position: { x: pos.x - NEW_W / 2, y: pos.y - NEW_H / 2 },
          data: { hookName, pendingEdgeFrom, onSave, onCancel },
          style: { width: NEW_W, height: NEW_H },
          draggable: true,
          selectable: false,
          zIndex: 1000,
        },
      ]);

      if (pendingEdgeFrom && pendingEdgeId) {
        setLocalEdges((prev) => [
          ...prev,
          {
            id: pendingEdgeId,
            source: pendingEdgeFrom,
            target: id,
            style: {
              stroke: "#00d4aa44",
              strokeWidth: 1.5,
              strokeDasharray: "4 2",
            },
          },
        ]);
      }
    },
    [send, setLocalEdges],
  );

  // Only allow connecting inline commands within the same hook (or into pending nodes)
  const isValidConnection = useCallback<IsValidConnection>(
    (conn) => {
      if (!conn.source || !conn.target) return false;
      if (conn.target.startsWith("__new__")) return true;
      const [srcHook, srcCmd] = conn.source.split("::");
      const [tgtHook, tgtCmd] = conn.target.split("::");
      if (srcHook !== tgtHook || tgtCmd === "__group" || srcCmd === tgtCmd) return false;
      const hook = hooks.find((h) => h.name === tgtHook);
      if (!hook) return false;
      const tgt = hook.commands.find((c) => !c.isRef && c.name === tgtCmd);
      return tgt !== undefined && !tgt.depends.includes(srcCmd);
    },
    [hooks],
  );

  const onConnect = useCallback(
    (conn: Connection) => {
      if (!conn.source || !conn.target) return;

      // Connecting to a pending node: draw a dashed preview edge only
      if (conn.target.startsWith("__new__")) {
        setLocalEdges((prev) =>
          addEdge(
            {
              source: conn.source!,
              target: conn.target!,
              id: `${conn.source}->${conn.target}`,
              style: { stroke: "#00d4aa44", strokeWidth: 1.5, strokeDasharray: "4 2" },
            },
            prev,
          ),
        );
        return;
      }

      const [hookName, srcCmd] = conn.source.split("::");
      const [, tgtCmd] = conn.target.split("::");
      const hook = hooks.find((h) => h.name === hookName);
      if (!hook) return;

      const newCmds = hook.commands.map((cmd) =>
        !cmd.isRef && cmd.name === tgtCmd
          ? { ...cmd, depends: [...cmd.depends, srcCmd] }
          : cmd,
      );

      setLocalEdges((prev) =>
        addEdge(
          {
            source: conn.source!,
            target: conn.target!,
            id: `${conn.source}->${conn.target}`,
            style: EDGE_STYLE,
          },
          prev,
        ),
      );

      send("hook.update", {
        hook: hookName,
        enabled: hook.enabled,
        parallel: hook.parallel,
        commands: newCmds,
      });
    },
    [hooks, send, setLocalEdges],
  );

  const onEdgesDelete = useCallback(
    (deleted: Edge[]) => {
      for (const edge of deleted) {
        if (edge.target.startsWith("__new__") || edge.id.includes("->pending-")) continue;
        const [hookName, srcCmd] = edge.source.split("::");
        const [, tgtCmd] = edge.target.split("::");
        const hook = hooks.find((h) => h.name === hookName);
        if (!hook) continue;

        const newCmds = hook.commands.map((cmd) =>
          !cmd.isRef && cmd.name === tgtCmd
            ? { ...cmd, depends: cmd.depends.filter((d) => d !== srcCmd) }
            : cmd,
        );

        send("hook.update", {
          hook: hookName,
          enabled: hook.enabled,
          parallel: hook.parallel,
          commands: newCmds,
        });
      }
    },
    [hooks, send],
  );

  // Pull source handle to empty canvas → spawn a new-command form node
  const onConnectEnd = useCallback(
    (
      event: MouseEvent | TouchEvent,
      connectionState: {
        isValid: boolean | null;
        fromNode: { id: string } | null;
      },
    ) => {
      if (connectionState.isValid) return;
      if (!connectionState.fromNode) return;
      const fromNodeId = connectionState.fromNode.id;
      if (fromNodeId.includes("__group") || fromNodeId.startsWith("__new__")) return;

      const clientX =
        "clientX" in event ? event.clientX : (event as TouchEvent).changedTouches[0].clientX;
      const clientY =
        "clientY" in event ? event.clientY : (event as TouchEvent).changedTouches[0].clientY;

      const pos = rfInstance?.screenToFlowPosition({ x: clientX, y: clientY });
      if (!pos) return;

      const [hookName] = fromNodeId.split("::");
      addNewCmdNode(pos, hookName, fromNodeId);
    },
    [rfInstance, addNewCmdNode],
  );

  // Palette drag-and-drop onto the canvas
  const onDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = "copy";
  }, []);

  const onDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      const type = e.dataTransfer.getData("application/githops-type");
      if (!type) return;
      const payload = e.dataTransfer.getData("application/githops-payload");

      const pos = rfInstance?.screenToFlowPosition({ x: e.clientX, y: e.clientY });
      if (!pos) return;

      // Determine which hook group received the drop
      const targetGroup = computedNodes.find(
        (n) =>
          n.type === "hookGroup" &&
          pos.x >= n.position.x &&
          pos.x <= n.position.x + (n.style?.width as number || 0) &&
          pos.y >= n.position.y &&
          pos.y <= n.position.y + (n.style?.height as number || 0),
      );
      if (!targetGroup) return;

      const hookName = targetGroup.data?.label as string;
      const hook = hooksRef.current.find((h) => h.name === hookName);
      if (!hook) return;

      if (type === "definition") {
        // Immediately append a definition-ref entry
        const newCmd: CommandEntry = {
          name: payload,
          run: "",
          test: false,
          isRef: true,
          refName: payload,
          depends: [],
          env: {},
        };
        send("hook.update", {
          hook: hookName,
          enabled: hook.enabled,
          parallel: hook.parallel,
          commands: [...hook.commands, newCmd],
        });
      } else {
        addNewCmdNode(pos, hookName);
      }
    },
    [rfInstance, computedNodes, send, addNewCmdNode],
  );

  if (!computedNodes.length) {
    return (
      <div className="flex items-center justify-center h-full text-[var(--color-muted)] text-sm">
        {t("no_flow")}
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col">
      <ReactFlow
        className="flex-1"
        nodes={allNodes}
        edges={localEdges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        onConnectEnd={onConnectEnd as any}
        onEdgesDelete={onEdgesDelete}
        onNodeClick={onNodeClick}
        onPaneClick={onPaneClick}
        isValidConnection={isValidConnection}
        nodeTypes={nodeTypes}
        onInit={setRfInstance}
        onDragOver={onDragOver}
        onDrop={onDrop}
        fitView
        fitViewOptions={{ padding: 0.15 }}
        nodesConnectable={true}
        deleteKeyCode={["Delete", "Backspace"]}
        proOptions={{ hideAttribution: true }}
      >
        <Background variant={BackgroundVariant.Dots} gap={24} size={1} color="#ffffff08" />
        <Controls showInteractive={false} />
        <MiniMap
          nodeColor={(n) => {
            if (n.type === "hookGroup") return "rgba(0,212,170,0.08)";
            if (n.type === "newCmdNode") return "#00d4aa22";
            const d = n.data as Partial<CmdData>;
            if (d.isInclude) return "#f9731644";
            if (d.hasOverrides) return "#c084fc44";
            if (d.isRef) return "#6496ff44";
            if (d.test) return "#ff884444";
            return "#00d4aa44";
          }}
          maskColor="#08081880"
        />
        <FlowPalette definitions={definitions} />
        {selectedNodeData && (
          <NodeDetailPanel data={selectedNodeData} onClose={() => setSelectedNodeId(null)} />
        )}
      </ReactFlow>
      {/* ── Legend + hint bar ── */}
      <div className="flex items-center justify-between px-4 py-1.5 border-t border-[var(--color-border)] shrink-0">
        <div className="flex items-center gap-4">
          <LegendItem color="#00d4aa" borderColor="#00d4aa55" bg="#00d4aa0d" label={t("legend_cmd")} />
          <LegendItem color="#ff8844" borderColor="#ff884450" bg="#ff88440d" label={t("legend_test")} />
          <LegendItem color="#6496ff" borderColor="#6496ff55" bg="#6496ff0d" label={t("legend_ref")} />
          <LegendItem color="#c084fc" borderColor="#c084fc55" bg="#c084fc0d" label={t("legend_ref_override")} />
          <LegendItem color="#f97316" borderColor="#f9731655" bg="#f973160d" label={t("legend_include")} />
          <LegendGroup />
        </div>
        <span className="text-[10px] text-[var(--color-muted)]">{t("flow_hint")}</span>
      </div>
    </div>
  );
}

function LegendItem({
  color,
  borderColor,
  bg,
  label,
}: {
  color: string;
  borderColor: string;
  bg: string;
  label: string;
}) {
  return (
    <div className="flex items-center gap-1.5">
      <div
        style={{
          width: 28,
          height: 16,
          background: bg,
          border: `1.5px solid ${borderColor}`,
          borderRadius: 4,
          flexShrink: 0,
        }}
      />
      <span style={{ fontSize: 10, color, fontFamily: '"SF Mono","Fira Code",monospace' }}>
        {label}
      </span>
    </div>
  );
}

function LegendGroup() {
  return (
    <div className="flex items-center gap-1.5">
      <div
        style={{
          width: 28,
          height: 16,
          background: "rgba(0,212,170,0.025)",
          border: "1px solid rgba(0,212,170,0.25)",
          borderRadius: 4,
          flexShrink: 0,
        }}
      />
      <span style={{ fontSize: 10, color: "rgba(0,212,170,0.6)", fontFamily: '"SF Mono","Fira Code",monospace' }}>
        {t("legend_hook")}
      </span>
    </div>
  );
}
