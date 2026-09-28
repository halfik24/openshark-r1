<script lang="ts">
  import { onMount } from "svelte";
  import * as THREE from "three";
  import {
    buildBody,
    buildEnvScene,
    buildShadow,
    buildSkins,
    buildUnderside,
    buildWheel,
  } from "$lib/mouseModel";

  let host: HTMLDivElement;
  let failed = $state(false);

  onMount(() => {
    let renderer: THREE.WebGLRenderer;
    try {
      renderer = new THREE.WebGLRenderer({
        antialias: true,
        alpha: true,
        powerPreference: "high-performance",
      });
    } catch {
      failed = true;
      return;
    }

    const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;

    renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
    renderer.toneMapping = THREE.ACESFilmicToneMapping;
    renderer.toneMappingExposure = 1.05;
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    renderer.domElement.style.width = "100%";
    renderer.domElement.style.height = "100%";
    renderer.domElement.style.display = "block";
    host.appendChild(renderer.domElement);

    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(30, 1, 0.1, 50);    camera.position.set(0, 1.45, 2.95);
    camera.lookAt(0, 0.08, 0);

    const pmrem = new THREE.PMREMGenerator(renderer);
    const envScene = buildEnvScene();
    const env = pmrem.fromScene(envScene, 0.03).texture;
    scene.environment = env;

    // группы: тень стоит на «поле», корпус крутится
    scene.add(buildShadow());

    const holder = new THREE.Group();
    scene.add(holder);

    const skins = buildSkins();
    // чёрный пластик: детали дают блики окружения, поэтому envMapIntensity
    // выше, чем было у серого корпуса
    const bodyMat = new THREE.MeshPhysicalMaterial({
      map: skins.base,
      color: 0xffffff,
      roughness: 0.42,
      metalness: 0.0,
      clearcoat: 0.5,
      clearcoatRoughness: 0.38,
      envMapIntensity: 0.9,
    });
    const body = new THREE.Mesh(buildBody(), bodyMat);
    holder.add(body);

    const wheel = buildWheel();
    holder.add(wheel);
    holder.add(buildUnderside());

    if (import.meta.env.DEV) {
        (window as unknown as Record<string, unknown>).__dbg = {
            scene,
            camera,
            renderer,
            holder,
            body,
            wheel,
        };
    }

    // мягкий свет поверх окружения
    const key = new THREE.DirectionalLight(0xffffff, 1.6);
    key.position.set(2.4, 3.4, 2.6);
    scene.add(key);
    const fill = new THREE.DirectionalLight(0x9ab8ff, 0.55);
    fill.position.set(-3, 1.4, 1.6);
    scene.add(fill);
    const rim = new THREE.DirectionalLight(0xffffff, 0.9);
    rim.position.set(-1.2, 2.2, -3);
    scene.add(rim);
    scene.add(new THREE.AmbientLight(0xffffff, 0.18));

    // ---- наклон за курсором
    const BASE_YAW = 0.6;
    let targetX = 0;
    let targetY = 0;
    let curX = 0;
    let curY = 0;

    const onPointer = (e: PointerEvent) => {
      if (reduced) return;
      const w = innerWidth;
      const h = innerHeight;
      targetX = (e.clientY / h - 0.5) * 2; // сверху вниз: -1..1
      targetY = (e.clientX / w - 0.5) * 2;
    };
    addEventListener("pointermove", onPointer, { passive: true });

    // колёсико крутится от прокрутки колесом
    let wheelSpin = 0;
    const onWheel = (e: WheelEvent) => {
      wheelSpin += e.deltaY * 0.012;
    };
    addEventListener("wheel", onWheel, { passive: true });

    // ---- resize
    const resize = () => {
      const w = host.clientWidth || 1;
      const h = host.clientHeight || 1;
      renderer.setSize(w, h, false);
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
    };
    const ro = new ResizeObserver(resize);
    ro.observe(host);
    resize();

    // ---- цикл: рисуем только пока есть изменение
    let raf = 0;
    let dirty = true;
    const tick = () => {
      raf = requestAnimationFrame(tick);
      if (document.hidden) return;

      const k = 0.12;
      const nx = curX + (targetX - curX) * k;
      const ny = curY + (targetY - curY) * k;
      const ws = wheel.rotation.x + (wheelSpin - wheel.rotation.x) * k;

      if (
        dirty ||
        Math.abs(nx - curX) > 1e-4 ||
        Math.abs(ny - curY) > 1e-4 ||
        Math.abs(ws - wheel.rotation.x) > 1e-4
      ) {
        curX = nx;
        curY = ny;
        wheel.rotation.x = ws;
        // pitch — вокруг X (кивок вверх-вниз), yaw — вокруг Y (поворот)
        holder.rotation.x = curX * 0.34;
        holder.rotation.y = BASE_YAW + curY * 0.5;
        holder.position.y = 0.02 + Math.abs(curX) * 0.02;
        renderer.render(scene, camera);
        dirty = false;
      }
    };
    tick();

    // защита от «встал навсегда» после возврата фокуса
    const onVisible = () => {
      dirty = true;
    };
    document.addEventListener("visibilitychange", onVisible);

    return () => {
      cancelAnimationFrame(raf);
      removeEventListener("pointermove", onPointer);
      removeEventListener("wheel", onWheel);
      document.removeEventListener("visibilitychange", onVisible);
      ro.disconnect();
      body.geometry.dispose();
      bodyMat.map?.dispose();
      bodyMat.dispose();
      envScene.traverse((o) => {
        const m = o as THREE.Mesh;
        m.geometry?.dispose();
        (m.material as THREE.Material | undefined)?.dispose();
      });
      env.dispose();
      pmrem.dispose();
      scene.traverse((o) => {
        const m = o as THREE.Mesh;
        if (m.geometry) m.geometry.dispose();
        const mat = m.material as THREE.Material | THREE.Material[] | undefined;
        if (Array.isArray(mat)) mat.forEach((x) => x.dispose());
        else mat?.dispose();
      });
      renderer.dispose();
      renderer.domElement.remove();
    };
  });
</script>

<div class="viewer" bind:this={host}>
  {#if failed}
    <div class="fallback">
      <span>🖱</span>
      <p>3D-просмотр недоступен (нет WebGL)</p>
    </div>
  {/if}
</div>

<style>
  .viewer {
    position: absolute;
    inset: 0;
    touch-action: none;
  }

  .fallback {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: #8e8e93;
    font-size: 13px;
  }

  .fallback span {
    font-size: 56px;
    filter: grayscale(1);
    opacity: 0.6;
  }
</style>
