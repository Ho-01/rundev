import { BatteryCharging, Braces, Brain, CodeXml, Coffee, Cpu, Focus, GitBranch, Keyboard, Layers, RotateCcw, Target, Terminal, Timer, Workflow, Zap } from "lucide-react";
import type { TraitId } from "../types/activity";
const core = { x: 420, y: 270 };
const primaryDistance = 120;
export const traitBoardNodes = [
  { id: "focus-ready", name: "몰입 준비", icon: Target, x: core.x, y: core.y - primaryDistance, description: "집중시간 XP 획득량 증가", traitId: "focus-ready" as TraitId },
  { id: "hot-keyboard", name: "뜨거운 키보드", icon: Keyboard, x: core.x + primaryDistance, y: core.y, description: "키보드 XP 획득량 증가", traitId: "hot-keyboard" as TraitId },
  { id: "reload", name: "재장전", icon: RotateCcw, x: core.x - primaryDistance, y: core.y, description: "매일 첫 집중 20분에 활동 보너스", traitId: "reload" as TraitId },
  ...[
    { id: "branch-s", icon: GitBranch, x: core.x, y: core.y + primaryDistance },
    { id: "branch-nw", icon: Brain, x: 280, y: 70 },
    { id: "branch-n", icon: Focus, x: 420, y: 32 },
    { id: "branch-ne", icon: Timer, x: 560, y: 70 },
    { id: "branch-w1", icon: Coffee, x: 140, y: 170 },
    { id: "branch-w2", icon: BatteryCharging, x: 140, y: 370 },
    { id: "branch-w", icon: Zap, x: 52, y: 270 },
    { id: "branch-e1", icon: CodeXml, x: 700, y: 170 },
    { id: "branch-e2", icon: Terminal, x: 700, y: 370 },
    { id: "branch-e", icon: Braces, x: 788, y: 270 },
    { id: "branch-sw", icon: Workflow, x: 300, y: 498 },
    { id: "branch-se", icon: Cpu, x: 540, y: 498 },
    { id: "branch-s2", icon: Layers, x: 420, y: 508 }
  ].map(node => ({ ...node, name: "확장 노드", description: "Coming soon", traitId: undefined }))
].map(node => ({ ...node, x: 420 + (node.x - 420) * .85, y: 270 + (node.y - 270) * .85 }));
export const traitBoardEdges = [
  ["core", "focus-ready"], ["core", "hot-keyboard"], ["core", "reload"], ["core", "branch-s"],
  ["focus-ready", "branch-nw"], ["focus-ready", "branch-n"], ["focus-ready", "branch-ne"],
  ["reload", "branch-w1"], ["reload", "branch-w2"], ["reload", "branch-w"],
  ["hot-keyboard", "branch-e1"], ["hot-keyboard", "branch-e2"], ["hot-keyboard", "branch-e"],
  ["branch-s", "branch-sw"], ["branch-s", "branch-se"], ["branch-s", "branch-s2"]
];
