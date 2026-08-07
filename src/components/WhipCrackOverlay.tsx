import {
  forwardRef,
  useEffect,
  useImperativeHandle,
  useRef
} from "react";
import {
  Bodies,
  Body,
  Composite,
  Constraint,
  Engine,
  type IBodyDefinition
} from "matter-js";

type Point = { x: number; y: number };
type CrackBurst = { x: number; y: number; life: number };

type WhipSimulation = {
  engine: Engine;
  bodies: Body[];
  handlePin: Constraint;
  targetPin: Constraint;
  handle: Point;
  target: Point;
  gestureMs: number;
  startedAt: number;
  lastFrameAt: number;
  impacted: boolean;
  frozen: boolean;
};

export type WhipCrackApi = {
  crackAt: (x: number, y: number, variant: "a" | "b") => void;
};

type Props = {
  width: number;
  height: number;
};

const PARTICLE_COUNT = 30;
const TOTAL_LENGTH = 224;
const REST_LENGTH = TOTAL_LENGTH / (PARTICLE_COUNT - 1);
const GESTURE_MS = 900;
const IMPACT_TIME = 0.72;
const FREEZE_TIME = 0.84;
const FIXED_STEP_MS = 1000 / 120;

function clamp(value: number, low: number, high: number) {
  return Math.max(low, Math.min(high, value));
}

function lerp(from: number, to: number, amount: number) {
  return from + (to - from) * amount;
}

function smoothstep(value: number) {
  const amount = clamp(value, 0, 1);
  return amount * amount * (3 - 2 * amount);
}

function createWhipSimulation(
  target: Point,
  width: number,
  now: number,
  reducedMotion: boolean
): WhipSimulation {
  const side = target.x <= width / 2 ? 1 : -1;
  const handle = {
    x: clamp(target.x + side * 236, 28, width - 28),
    y: clamp(target.y + 12, 36, 110)
  };
  const engine = Engine.create({
    gravity: { x: 0, y: 0.12, scale: 0.001 },
    positionIterations: 12,
    velocityIterations: 10,
    constraintIterations: 8
  });
  const bodies: Body[] = [];

  for (let index = 0; index < PARTICLE_COUNT; index++) {
    const amount = index / (PARTICLE_COUNT - 1);
    const radius = lerp(3.2, 0.75, amount);
    const x = lerp(handle.x, target.x, amount);
    const y =
      lerp(handle.y, target.y, amount) -
      Math.sin(amount * Math.PI) * 4;
    const options: IBodyDefinition = {
      friction: 0,
      frictionAir: lerp(0.012, 0.003, amount),
      restitution: 0,
      collisionFilter: { category: 0x0002, mask: 0 }
    };
    const body = Bodies.circle(x, y, radius, options);
    Body.setMass(body, lerp(1.15, 0.055, amount ** 1.5));
    bodies.push(body);
  }

  const constraints: Constraint[] = [];
  for (let index = 0; index < bodies.length - 1; index++) {
    constraints.push(
      Constraint.create({
        bodyA: bodies[index],
        bodyB: bodies[index + 1],
        length: REST_LENGTH,
        stiffness: 0.985,
        damping: 0.045
      })
    );
  }

  // OpenWhip keeps the handle end relatively rigid and lets the tip bend
  // freely. Two-link constraints reproduce that stiffness gradient while
  // Matter owns the integration and distance solving.
  for (let index = 0; index < bodies.length - 2; index++) {
    const amount = index / (bodies.length - 3);
    constraints.push(
      Constraint.create({
        bodyA: bodies[index],
        bodyB: bodies[index + 2],
        length: REST_LENGTH * 1.94,
        stiffness: lerp(0.62, 0.025, amount),
        damping: 0.025
      })
    );
  }

  const handlePin = Constraint.create({
    pointA: { ...handle },
    bodyB: bodies[0],
    length: 0,
    stiffness: 1,
    damping: 0.18
  });
  const targetPin = Constraint.create({
    pointA: { ...target },
    bodyB: bodies[bodies.length - 1],
    length: 0,
    stiffness: 0,
    damping: 0.2
  });

  Composite.add(engine.world, [
    ...bodies,
    ...constraints,
    handlePin,
    targetPin
  ]);

  return {
    engine,
    bodies,
    handlePin,
    targetPin,
    handle,
    target,
    gestureMs: reducedMotion ? 420 : GESTURE_MS,
    startedAt: now,
    lastFrameAt: now,
    impacted: false,
    frozen: false
  };
}

function updateHandle(simulation: WhipSimulation, time: number) {
  const { handle, target } = simulation;
  const axisX = handle.x - target.x;
  const axisY = handle.y - target.y;
  const axisLength = Math.hypot(axisX, axisY);
  const alongX = axisX / axisLength;
  const alongY = axisY / axisLength;
  const normalX = -alongY;
  const normalY = alongX;
  let normalOffset = 0;
  let alongOffset = 0;

  if (time < 0.24) {
    const windup = smoothstep(time / 0.24);
    normalOffset = 82 * windup;
    alongOffset = 12 * windup;
  } else if (time < IMPACT_TIME) {
    // Reversing the handle across the whip axis launches a transverse
    // wave. The tapered mass then accelerates that wave toward the tip.
    const cast = smoothstep(
      (time - 0.24) / (IMPACT_TIME - 0.24)
    );
    normalOffset = lerp(82, -76, cast);
    alongOffset = lerp(12, -38, cast);
  } else {
    normalOffset = -76;
    alongOffset = -38;
  }

  simulation.handlePin.pointA = {
    x: handle.x + normalX * normalOffset + alongX * alongOffset,
    y: handle.y + normalY * normalOffset + alongY * alongOffset
  };
}

function stepSimulation(simulation: WhipSimulation, now: number) {
  const time = clamp(
    (now - simulation.startedAt) / simulation.gestureMs,
    0,
    1
  );
  updateHandle(simulation, time);

  if (!simulation.impacted && time >= IMPACT_TIME) {
    simulation.impacted = true;
    simulation.targetPin.stiffness = 1;
    const tip = simulation.bodies[simulation.bodies.length - 1];
    Body.setPosition(tip, simulation.target);
    Body.setVelocity(tip, { x: 0, y: 0 });
  }

  if (!simulation.frozen) {
    const elapsed = clamp(now - simulation.lastFrameAt, 0, 34);
    let remaining = elapsed;
    while (remaining > 0) {
      const step = Math.min(FIXED_STEP_MS, remaining);
      Engine.update(simulation.engine, step);
      remaining -= step;
    }
    simulation.lastFrameAt = now;
  }

  if (time >= FREEZE_TIME) simulation.frozen = true;
  return time;
}

function catmullPoint(points: Point[], index: number) {
  if (index < 0) {
    return {
      x: 2 * points[0].x - points[1].x,
      y: 2 * points[0].y - points[1].y
    };
  }
  if (index >= points.length) {
    const end = points.length - 1;
    return {
      x: 2 * points[end].x - points[end - 1].x,
      y: 2 * points[end].y - points[end - 1].y
    };
  }
  return points[index];
}

function segmentBezier(points: Point[], index: number) {
  const p0 = catmullPoint(points, index - 1);
  const p1 = points[index];
  const p2 = points[index + 1];
  const p3 = catmullPoint(points, index + 2);
  return {
    cp1x: p1.x + (p2.x - p0.x) / 6,
    cp1y: p1.y + (p2.y - p0.y) / 6,
    cp2x: p2.x - (p3.x - p1.x) / 6,
    cp2y: p2.y - (p3.y - p1.y) / 6,
    x: p2.x,
    y: p2.y
  };
}

function strokeWhip(
  context: CanvasRenderingContext2D,
  points: Point[],
  color: string,
  handleWidth: number,
  tipWidth: number
) {
  for (let index = 0; index < points.length - 1; index++) {
    const amount = index / (points.length - 2);
    const curve = segmentBezier(points, index);
    context.strokeStyle = color;
    context.lineWidth = lerp(handleWidth, tipWidth, amount);
    context.beginPath();
    context.moveTo(points[index].x, points[index].y);
    context.bezierCurveTo(
      curve.cp1x,
      curve.cp1y,
      curve.cp2x,
      curve.cp2y,
      curve.x,
      curve.y
    );
    context.stroke();
  }
}

function drawWhip(
  context: CanvasRenderingContext2D,
  bodies: Body[],
  opacity: number
) {
  const points = bodies.map(({ position }) => ({
    x: position.x,
    y: position.y
  }));
  context.save();
  context.globalAlpha = opacity;
  context.lineCap = "round";
  context.lineJoin = "round";
  strokeWhip(context, points, "rgba(0, 0, 0, .94)", 8.4, 3);
  strokeWhip(context, points, "#f4f3ef", 5.7, 1.05);
  context.restore();
}

function drawBurst(
  context: CanvasRenderingContext2D,
  burst: CrackBurst
) {
  const radius = 7 + (1 - burst.life) * 19;
  context.strokeStyle = `rgba(255, 221, 120, ${burst.life})`;
  context.lineWidth = 1.4;
  for (let index = 0; index < 6; index++) {
    const angle = (Math.PI * 2 * index) / 6;
    context.beginPath();
    context.moveTo(
      burst.x + Math.cos(angle) * 4,
      burst.y + Math.sin(angle) * 4
    );
    context.lineTo(
      burst.x + Math.cos(angle) * radius,
      burst.y + Math.sin(angle) * radius
    );
    context.stroke();
  }
}

export const WhipCrackOverlay = forwardRef<WhipCrackApi, Props>(
  function WhipCrackOverlay({ width, height }, ref) {
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const contextRef = useRef<CanvasRenderingContext2D | null>(null);
    const simulationRef = useRef<WhipSimulation | null>(null);
    const burstsRef = useRef<CrackBurst[]>([]);
    const animationFrameRef = useRef(0);
    const renderRef = useRef<(now: number) => void>(() => {});
    const reducedMotionRef = useRef(false);

    useImperativeHandle(ref, () => ({
      crackAt(x, y, _variant) {
        const target = {
          x: clamp(x, 4, width - 4),
          y: clamp(y, 4, height - 4)
        };
        const simulation = createWhipSimulation(
          target,
          width,
          performance.now(),
          reducedMotionRef.current
        );
        simulationRef.current = simulation;
        burstsRef.current = [];

        // Paint the initial frame synchronously. Some WebView2 builds can
        // defer the first requestAnimationFrame while the Tauri popover is
        // gaining focus, which previously made a whole short gesture vanish.
        const context =
          contextRef.current ?? canvasRef.current?.getContext("2d") ?? null;
        if (context) {
          contextRef.current = context;
          context.clearRect(0, 0, width, height);
          drawWhip(context, simulation.bodies, 1);
        }
        if (animationFrameRef.current === 0) {
          animationFrameRef.current = requestAnimationFrame((now) => renderRef.current(now));
        }
      }
    }));

    useEffect(() => {
      const media = window.matchMedia?.("(prefers-reduced-motion: reduce)");
      if (!media) return;
      reducedMotionRef.current = media.matches;
      const onChange = () => {
        reducedMotionRef.current = media.matches;
      };
      media.addEventListener("change", onChange);
      return () => media.removeEventListener("change", onChange);
    }, []);

    useEffect(() => {
      const canvas = canvasRef.current;
      if (
        !canvas ||
        typeof CanvasRenderingContext2D === "undefined"
      ) return;
      const context = canvas.getContext("2d");
      if (!context) return;
      contextRef.current = context;

      const render = (now: number) => {
        animationFrameRef.current = 0;
        context.clearRect(0, 0, width, height);
        const simulation = simulationRef.current;

        if (simulation) {
          const wasImpacted = simulation.impacted;
          const time = stepSimulation(simulation, now);
          if (!wasImpacted && simulation.impacted) {
            burstsRef.current.push({
              ...simulation.target,
              life: 1
            });
          }
          const opacity =
            time > 0.9 ? 1 - smoothstep((time - 0.9) / 0.1) : 1;
          drawWhip(context, simulation.bodies, opacity);
          if (time >= 1) simulationRef.current = null;
        }

        burstsRef.current = burstsRef.current
          .map((burst) => ({ ...burst, life: burst.life - 0.075 }))
          .filter((burst) => burst.life > 0);
        for (const burst of burstsRef.current) drawBurst(context, burst);

        if (simulationRef.current || burstsRef.current.length > 0) {
          animationFrameRef.current = requestAnimationFrame(render);
        }
      };

      renderRef.current = render;
      return () => {
        contextRef.current = null;
        if (animationFrameRef.current !== 0) {
          cancelAnimationFrame(animationFrameRef.current);
          animationFrameRef.current = 0;
        }
      };
    }, [height, width]);

    return (
      <canvas
        ref={canvasRef}
        className="whip-crack-canvas"
        width={width}
        height={height}
        aria-hidden="true"
      />
    );
  }
);
