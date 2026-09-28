import * as THREE from "three";

/** Реальные габариты Attack Shark R1: 123.5 x 64 x 41 мм. */
export const SIZE = { L: 1.235, W: 0.64, H: 0.41 };

/**
 * Профиль высоты вдоль длины: t=0 нос, t=1 зад.
 * Замер по чертежу производителя (доля от 41 мм): нос уже не «треугольник»,
 * горб стоит посредине, зад кончается тонким закруглением.
 */
const H_PTS: [number, number][] = [
    [0.0, 0.43],
    [0.08, 0.59],
    [0.16, 0.77],
    [0.24, 0.84],
    [0.36, 0.95],
    [0.5, 1.0],
    [0.6, 0.98],
    [0.72, 0.87],
    [0.86, 0.63],
    [1.0, 0.19],
];

/**
 * Профиль ширины: замер по фото вида сверху (L/W = 1.93 — ровно как в
 * спецификации, значит фото без искажений). Спереди талия, максимум —
 * в задней трети, нос и зад заметно сужены.
 */
const W_PTS: [number, number][] = [
    [0.0, 0.18],
    [0.04, 0.69],
    [0.09, 0.93],
    [0.14, 0.945],
    [0.24, 0.92],
    [0.34, 0.913],
    [0.5, 0.93],
    [0.62, 0.975],
    [0.71, 1.0],
    [0.8, 0.965],
    [0.88, 0.85],
    [0.95, 0.6],
    [1.0, 0.06],
];

/**
 * Неравномерная кубическая Эрмита по опорным точкам: касательные считаются
 * по соседям, поэтому нет «плоских» участков на узлах (от них корпус
 * покрывается поперечной рябью).
 */
function profile(pts: [number, number][], t: number): number {
    const n = pts.length;
    if (t <= pts[0][0]) return pts[0][1];
    if (t >= pts[n - 1][0]) return pts[n - 1][1];
    for (let i = 0; i < n - 1; i++) {
        const x0 = pts[i][0];
        const y0 = pts[i][1];
        const x1 = pts[i + 1][0];
        const y1 = pts[i + 1][1];
        if (t < x0 || t > x1) continue;

        const dx = x1 - x0;
        const dSecant = (y1 - y0) / dx;
        const m0 =
            i > 0 ? (y1 - pts[i - 1][1]) / (x1 - pts[i - 1][0]) : dSecant;
        const m1 =
            i + 2 < n ? (pts[i + 2][1] - y0) / (pts[i + 2][0] - x0) : dSecant;

        const s = (t - x0) / dx;
        const s2 = s * s;
        const s3 = s2 * s;
        return (
            (2 * s3 - 3 * s2 + 1) * y0 +
            (s3 - 2 * s2 + s) * dx * m0 +
            (-2 * s3 + 3 * s2) * y1 +
            (s3 - s2) * dx * m1
        );
    }
    return pts[n - 1][1];
}

/**
 * Днище приподнимается у носа и зада: на чертеже низ до крайних точек не
 * доходит — там корпус подрыт, мышь стоит на полозках.
 */
function bottomLift(t: number): number {
    // фронт по чертежу: подъём 0.385H в кончике, ноль производной на t=0.16
    const front = t < 0.16 ? Math.pow((0.16 - t) / 0.16, 2) : 0;
    // зад по чертежу: хвост «подрыт» с t=0.86, квадратно-кубический рост до 0.146H
    const rear = t > 0.86 ? Math.pow((t - 0.86) / 0.14, 3) : 0;
    return 0.385 * front + 0.146 * rear;
}

/** Точка поперечного сечения под углом θ (0 — правая кромка дна, π/2 — верх). */
function sectionPoint(
    theta: number,
    w: number,
    h: number,
    lift: number,
    hb: number,
    n: number
): [number, number] {
    const p = 2 / n;
    const ca = Math.cos(theta);
    const sa = Math.sin(theta);
    const x = w * Math.sign(ca) * Math.pow(Math.abs(ca), p);
    const y =
        sa >= 0
            ? lift + hb + (h - lift - hb) * Math.pow(sa, p)
            : lift + hb * (1 - Math.pow(-sa, 0.65));
    return [x, y];
}

/**
 * Развёртка свода по длине дуги, а не по углу.
 *
 * При угловой развёртке у плоского верха один пиксель текстуры «весит»
 * ~1.3 мм (шов между кнопками получается толще пикселя и мылится), а у
 * нижних кромок — 0.03 мм, разрешение уходит впустую. С равномерной длиной
 * дуги всё рисуется в миллиметрах: ~8.6 пикселя на мм.
 */
function arcLut(
    w: number,
    h: number,
    lift: number,
    hb: number,
    n: number
): Float64Array {
    const S = 48;
    const lut = new Float64Array(S + 1);
    let acc = 0;
    let px = w;
    let py = lift + hb;
    for (let i = 1; i <= S; i++) {
        const [x, y] = sectionPoint((i / S) * (Math.PI / 2), w, h, lift, hb, n);
        acc += Math.hypot(x - px, y - py);
        lut[i] = acc;
        px = x;
        py = y;
    }
    for (let i = 0; i <= S; i++) lut[i] /= acc;
    return lut;
}

function lutAt(lut: Float64Array, k: number): number {
    const x = k * (lut.length - 1);
    const i = Math.min(lut.length - 2, Math.floor(x));
    return lut[i] + (lut[i + 1] - lut[i]) * (x - i);
}

/**
 * Тело мыши: поперечные сверхэллиптические сечения, натянутые вдоль длины.
 * Параметр идёт от правого нижнего края (u=0) через верх (u=0.25) к левому
 * нижнему (u=0.5); u=0.5..1 — плоское днище.
 */
export function buildBody(): THREE.BufferGeometry {
    const SEG_U = 128;
    const SEG_V = 96;
    const hb = 0.03 * SIZE.H; // радиус скругления низа боковины
    const n = 3.6; // показатель сверхэллипса: 2 — эллипс, больше — «корпуснее»
    const pos: number[] = [];
    const uv: number[] = [];
    const idx: number[] = [];

    for (let iv = 0; iv <= SEG_V; iv++) {
        const t = iv / SEG_V; // ровно от носа (z=+L/2) до зада (z=-L/2)
        const h = SIZE.H * profile(H_PTS, t);
        const w = (SIZE.W / 2) * profile(W_PTS, t);
        const lift = SIZE.H * bottomLift(t);
        const z = (0.5 - t) * SIZE.L;
        const lut = arcLut(w, h, lift, hb, n);

        for (let iu = 0; iu <= SEG_U; iu++) {
            const a = (iu / SEG_U) * Math.PI * 2;
            const [x, y] = sectionPoint(a, w, h, lift, hb, n);

            // свод (a <= π) разворачиваем по длине дуги, днище — по углу
            let uu: number;
            if (a <= Math.PI) {
                const half = Math.PI / 2;
                const k = a <= half ? a / half : (Math.PI - a) / half;
                const fr = lutAt(lut, k);
                uu = a <= half ? 0.25 * fr : 0.5 - 0.25 * fr;
            } else {
                uu = a / (Math.PI * 2);
            }

            pos.push(x, y, z);
            uv.push(uu, t);
        }
    }

    const row = SEG_U + 1;
    for (let iv = 0; iv < SEG_V; iv++) {
        for (let iu = 0; iu < SEG_U; iu++) {
            const a0 = iv * row + iu;
            const b0 = a0 + 1;
            const a1 = a0 + row;
            const b1 = a1 + 1;
            idx.push(a0, a1, b0, b0, a1, b1);
        }
    }

    addCap(pos, uv, idx, 0, true, SEG_U, SEG_V);
    addCap(pos, uv, idx, SEG_V, false, SEG_U, SEG_V);

    const geo = new THREE.BufferGeometry();
    geo.setAttribute("position", new THREE.Float32BufferAttribute(pos, 3));
    geo.setAttribute("uv", new THREE.Float32BufferAttribute(uv, 2));
    geo.setIndex(idx);
    geo.computeVertexNormals();
    return geo;
}

/** Закрывает отверстие на носу/заде веером с лёгким «обтыком». */
function addCap(
    pos: number[],
    uv: number[],
    idx: number[],
    iv: number,
    front: boolean,
    segU: number,
    segV: number
) {
    const row = segU + 1;
    const start = iv * row;

    const z = pos[(start * 3) + 2];
    let ySum = 0;
    for (let i = 0; i < row; i++) ySum += pos[((start + i) * 3) + 1];
    const yMid = ySum / row;
    const bulge = front ? 0.008 : -0.01;

    const center = pos.length / 3;
    pos.push(0, yMid, z + bulge);
    uv.push(0.25, iv / segV);

    for (let i = 0; i < segU; i++) {
        const a = start + i;
        const b = start + i + 1;
        if (front) idx.push(center, a, b);
        else idx.push(center, b, a);
    }
}

/** Колёсико прокрутки: ось along X, накатка — тёмный силикон. */
export function buildWheel(): THREE.Mesh {
    const geo = new THREE.CylinderGeometry(0.065, 0.065, 0.062, 32, 1);
    geo.rotateZ(Math.PI / 2);
    const mat = new THREE.MeshPhysicalMaterial({
        color: 0x14161a,
        roughness: 0.85,
        metalness: 0,
    });
    const mesh = new THREE.Mesh(geo, mat);

    const t = 0.19; // центр колеса по фото (~19% длины от носа)
    const h = SIZE.H * profile(H_PTS, t);
    mesh.position.set(0, h - 0.05, (0.5 - t) * SIZE.L); // выступает ~1.5 мм
    return mesh;
}

/** Тефлоновые ножки и датчик снизу. */
export function buildUnderside(): THREE.Group {
    const g = new THREE.Group();

    const footMat = new THREE.MeshPhysicalMaterial({
        color: 0xdedfe4,
        roughness: 0.7,
        metalness: 0,
    });
    const foot = (w: number, d: number, x: number, z: number, rot = 0) => {
        const m = new THREE.Mesh(new THREE.BoxGeometry(w, 0.024, d), footMat);
        m.position.set(x, 0, z);
        m.rotation.y = rot;
        g.add(m);
    };
    foot(0.15, 0.13, 0.17, 0.38, -0.14);
    foot(0.15, 0.13, -0.17, 0.38, 0.14);
    foot(0.44, 0.15, 0, -0.44);

    const sensor = new THREE.Mesh(
        new THREE.CylinderGeometry(0.036, 0.036, 0.02, 24),
        new THREE.MeshPhysicalMaterial({
            color: 0x07070a,
            roughness: 0.35,
            metalness: 0.1,
        })
    );
    sensor.position.set(0, 0, 0.02);
    g.add(sensor);
    return g;
}

/** Мягкая тень-пятно под мышью (не крутится вместе с корпусом). */
export function buildShadow(): THREE.Mesh {
    const c = document.createElement("canvas");
    c.width = c.height = 256;
    const ctx = c.getContext("2d")!;
    const grad = ctx.createRadialGradient(128, 128, 10, 128, 128, 126);
    grad.addColorStop(0, "rgba(0,0,0,0.72)");
    grad.addColorStop(0.55, "rgba(0,0,0,0.34)");
    grad.addColorStop(1, "rgba(0,0,0,0)");
    ctx.fillStyle = grad;
    ctx.fillRect(0, 0, 256, 256);

    const tex = new THREE.CanvasTexture(c);
    const mesh = new THREE.Mesh(
        new THREE.PlaneGeometry(1.7, 1.25),
        new THREE.MeshBasicMaterial({
            map: tex,
            transparent: true,
            depthWrite: false,
        })
    );
    mesh.rotation.x = -Math.PI / 2;
    mesh.position.y = -0.014;
    return mesh;
}

/**
 * Окружение-«студия»: купол с градиентом по высоте и три световые панели.
 *
 * Градиент считается в float и сразу уходит PMREM в half-float — ступени
 * 8-битного canvas-градиента не существуют, а именно они давали волны на
 * зеркальном лаке корпуса.
 */
export function buildEnvScene(): THREE.Scene {
    const s = new THREE.Scene();

    const R = 30;
    const geo = new THREE.SphereGeometry(R, 48, 32);
    const top = new THREE.Color(0.38, 0.41, 0.48);
    const mid = new THREE.Color(0.13, 0.15, 0.18);
    const bot = new THREE.Color(0.03, 0.03, 0.035);
    const colors: number[] = [];
    const pos = geo.getAttribute("position");
    const tmp = new THREE.Color();
    for (let i = 0; i < pos.count; i++) {
        const y = pos.getY(i) / R; // -1..1
        if (y > 0) tmp.lerpColors(mid, top, y);
        else tmp.lerpColors(bot, mid, y + 1);
        colors.push(tmp.r, tmp.g, tmp.b);
    }
    geo.setAttribute("color", new THREE.Float32BufferAttribute(colors, 3));
    s.add(
        new THREE.Mesh(
            geo,
            new THREE.MeshBasicMaterial({
                vertexColors: true,
                side: THREE.BackSide,
            })
        )
    );

    // световые панели: цвет можно держать >1, PMREM сохраняет это в HDR
    const panel = (
        w: number,
        h: number,
        p: [number, number, number],
        c: [number, number, number]
    ) => {
        const m = new THREE.Mesh(
            new THREE.PlaneGeometry(w, h),
            new THREE.MeshBasicMaterial({
                color: new THREE.Color(c[0], c[1], c[2]),
                side: THREE.DoubleSide,
            })
        );
        m.position.set(p[0], p[1], p[2]);
        m.lookAt(0, 0, 0);
        s.add(m);
        return m;
    };

    panel(13, 7, [-7, 9, 7], [2.6, 2.6, 2.8]); // ключевой софтбокс
    panel(8, 8, [10, 3, 5], [1.1, 1.3, 1.8]); // холодный заполняющий
    panel(7, 5, [-3, 5, -10], [1.6, 1.6, 1.9]); // контровой
    panel(20, 20, [0, -14, 0], [0.08, 0.08, 0.1]); // тёмный «пол»
    return s;
}

const U_TOP = 0.25; // верх корпуса в UV (по длине дуги — ровно верх)
const U_SEAM = 0.129; // где поперечный шов уходит на бок
const T_WHEEL_F = 0.122; // перед колёсиком
const T_WHEEL_R = 0.258; // за колёсиком
const T_PILL_F = 0.32; // DPI-кнопка: контурная пилюля
const T_PILL_R = 0.41;
const U_BTN = 0.397; // верх боковой кнопки (левая стенка)
const U_BTN_H = 0.0315; // её высота по дуге (~7.5 мм)

/** Вид сверху: поперечный шов — дуга, дальше всего назад он по центру. */
function deckV(u: number): number {
    const k = (u - U_TOP) / (U_TOP - U_SEAM);
    return 0.5 - 0.06 * k * k;
}

/** Кожа корпуса: база + шум + швы + колесо + DPI + боковые кнопки + бок. */
export function buildSkins(): { base: THREE.CanvasTexture } {
    const W = 2048;
    const H = 1024;
    const c = document.createElement("canvas");
    c.width = W;
    c.height = H;
    const ctx = c.getContext("2d")!;

    // чёрный матовый пластик
    ctx.fillStyle = "#1e2025";
    ctx.fillRect(0, 0, W, H);

    // мелкое зерно пластика
    for (let i = 0; i < 9000; i++) {
        const x = Math.random() * W;
        const y = Math.random() * H;
        const a = Math.random();
        ctx.fillStyle =
            a > 0.5 ? "rgba(255,255,255,0.02)" : "rgba(0,0,0,0.05)";
        ctx.fillRect(x, y, 2, 2);
    }

    const CX = U_TOP * W;
    ctx.fillStyle = "#0a0b0e";

    // периметр стыка верхней крышки с днищем (u=0.5 и u=1 — нижние кромки)
    ctx.fillRect(0.5 * W - 3, 0, 6, H);
    ctx.fillRect(W - 3, 0, 6, H);

    // поперечный шов крышки: дуга за DPI-пилюлей, концы уходят на бок
    ctx.strokeStyle = "#0a0b0e";
    ctx.lineWidth = 7;
    ctx.beginPath();
    for (let i = 0; i <= 96; i++) {
        const u = U_SEAM + 2 * (U_TOP - U_SEAM) * (i / 96);
        const x = u * W;
        const y = deckV(u) * H;
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
    }
    ctx.stroke();
    ctx.strokeStyle = "rgba(255,255,255,0.07)";
    ctx.lineWidth = 2;
    ctx.save();
    ctx.translate(0, 6);
    ctx.stroke();
    ctx.restore();
    ctx.fillStyle = "#0a0b0e";

    // продольный шов между L/R кнопками: нос -> колесо -> пилюля -> шов
    ctx.fillRect(CX - 3.5, 0, 7, T_WHEEL_F * H);
    ctx.fillRect(CX - 3.5, T_WHEEL_R * H, 7, (T_PILL_F - T_WHEEL_R) * H);
    ctx.fillRect(CX - 3.5, T_PILL_R * H, 7, (deckV(U_TOP) - T_PILL_R) * H);

    // ниша колёсика: 9 мм по дуге, 17 мм вдоль
    const slotW = 77;
    roundRect(
        ctx,
        CX - slotW / 2,
        T_WHEEL_F * H,
        slotW,
        (T_WHEEL_R - T_WHEEL_F) * H,
        26
    );
    ctx.fill();
    ctx.strokeStyle = "rgba(255,255,255,0.12)";
    ctx.lineWidth = 3;
    ctx.stroke();

    // DPI-кнопка — контурная пилюля на центре (6 x 11 мм)
    ctx.beginPath();
    ctx.ellipse(
        CX,
        ((T_PILL_F + T_PILL_R) / 2) * H,
        25,
        ((T_PILL_R - T_PILL_F) / 2) * H,
        0,
        0,
        Math.PI * 2
    );
    ctx.fillStyle = "rgba(255,255,255,0.03)";
    ctx.fill();
    ctx.strokeStyle = "#4b4f59";
    ctx.lineWidth = 4;
    ctx.stroke();

    // боковые кнопки: на реальной мыши t = 0.30..0.62, а не у носа
    const sideBtn = (v0: number, v1: number) => {
        ctx.fillStyle = "#15171c";
        roundRect(ctx, U_BTN * W, v0 * H, U_BTN_H * W, (v1 - v0) * H, 22);
        ctx.fill();
        ctx.strokeStyle = "rgba(255,255,255,0.17)";
        ctx.lineWidth = 3;
        ctx.stroke();
    };
    sideBtn(0.3, 0.44);
    sideBtn(0.46, 0.62);

    const base = new THREE.CanvasTexture(c);
    base.flipY = false;
    base.colorSpace = THREE.SRGBColorSpace;
    base.anisotropy = 8;

    // логотип — на левом боку под кнопками; на реальной мыши подсветки
    // и надписи сверху нет
    const LX = 0.4515 * W;
    ctx.save();
    ctx.translate(LX, 0.275 * H);
    // rotate(-π/2): базовая линия идёт вдоль длины (зад → нос) — так
    // надпись читается с левого бока снизу вверх, а буквы смотрят
    // вершиной к верху стенки
    ctx.rotate(-Math.PI / 2);
    ctx.font = "600 30px system-ui, sans-serif";
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    ctx.fillStyle = "#787c86";
    ctx.fillText("ATTACK SHARK", 0, 0);
    ctx.restore();

    // акулья печать — перед надписью (ближе к хвосту)
    ctx.save();
    ctx.translate(LX, 0.4 * H);
    ctx.rotate(-Math.PI / 2);
    ctx.fillStyle = "#787c86";
    ctx.beginPath();
    ctx.moveTo(-22, 14);
    ctx.quadraticCurveTo(-8, -4, 3, -20);
    ctx.quadraticCurveTo(9, -2, 22, 14);
    ctx.closePath();
    ctx.fill();
    ctx.restore();

    base.needsUpdate = true;

    return { base };
}

function roundRect(
    ctx: CanvasRenderingContext2D,
    x: number,
    y: number,
    w: number,
    h: number,
    r: number
) {
    const rr = Math.min(r, w / 2, h / 2);
    ctx.beginPath();
    ctx.moveTo(x + rr, y);
    ctx.arcTo(x + w, y, x + w, y + h, rr);
    ctx.arcTo(x + w, y + h, x, y + h, rr);
    ctx.arcTo(x, y + h, x, y, rr);
    ctx.arcTo(x, y, x + w, y, rr);
    ctx.closePath();
}
