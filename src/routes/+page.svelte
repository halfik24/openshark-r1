<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import RibbonGlow from "$lib/RibbonGlow.svelte";

  type Config = {
    polling_rate: number;
    dpis: number[];
    active_dpi: number;
    sleep_time: number;
    deep_sleep_time: number;
    key_response_time: number;
    ripple_control: boolean;
    angle_snap: boolean;
  };

  const RATES = [125, 250, 500, 1000];

  let config = $state<Config>({
    polling_rate: 125,
    dpis: [800, 1600, 3200, 4000, 5000, 12000],
    active_dpi: 3,
    sleep_time: 6,
    deep_sleep_time: 12,
    key_response_time: 4,
    ripple_control: false,
    angle_snap: false,
  });

  let status = $state<{ kind: "ok" | "err" | "busy" | "idle"; text: string }>({
    kind: "idle",
    text: "",
  });
  let batteryText = $state("…");
  let batteryKind = $state<"load" | "cable" | "ok" | "low" | "off">("load");

  async function load() {
    status = { kind: "busy", text: "Читаем настройки с мыши…" };
    try {
      const [cfg, onDisk] = await invoke<[Config, boolean]>("get_config");
      config = cfg;
      status = {
        kind: "idle",
        text: onDisk
          ? "Загружено из config.json"
          : "Файла пока нет, значения по умолчанию",
      };
    } catch (e) {
      status = { kind: "err", text: `Не удалось прочитать настройки: ${e}` };
    }
    refreshBattery();
  }

  async function refreshBattery() {
    try {
      const level = await invoke<number | null>("battery");
      if (level === null) {
        batteryText = "кабель";
        batteryKind = "cable";
      } else {
        batteryText = `${level}%`;
        batteryKind = level < 20 ? "low" : "ok";
      }
    } catch (e) {
      batteryText = typeof e === "string" && e ? e : "нет связи";
      batteryKind = "off";
    }
  }

  async function apply() {
    status = { kind: "busy", text: "Отправляем настройки…" };
    try {
      // Отклик кнопки обязан быть чётным, 4..50 мс.
      config.key_response_time = Math.max(
        4,
        Math.min(50, config.key_response_time - (config.key_response_time % 2)),
      );
      await invoke("apply_config", { config });
      status = { kind: "ok", text: "Настройки применены" };
    } catch (e) {
      status = { kind: "err", text: `Не удалось применить настройки: ${e}` };
    }
  }

  async function resetButtons() {
    status = { kind: "busy", text: "Восстанавливаем кнопки…" };
    try {
      await invoke("reset_buttons");
      status = { kind: "ok", text: "Кнопки возвращены к заводским" };
    } catch (e) {
      status = { kind: "err", text: `Не удалось восстановить кнопки: ${e}` };
    }
  }

  function resetDefaults() {
    config = {
      polling_rate: 125,
      dpis: [800, 1600, 3200, 4000, 5000, 12000],
      active_dpi: 3,
      sleep_time: 6,
      deep_sleep_time: 12,
      key_response_time: 4,
      ripple_control: false,
      angle_snap: false,
    };
    status = { kind: "idle", text: "Подставлены значения по умолчанию" };
  }

  const activeDpi = $derived(config.dpis[config.active_dpi - 1] ?? 0);

  load();
</script>

{#snippet seg(label: string, value: number)}
  <button
    class:on={config.polling_rate === value}
    onclick={() => (config.polling_rate = value)}
  >
    {label}
  </button>
{/snippet}

{#snippet toggle(key: "ripple_control" | "angle_snap", label: string)}
  <button
    class="row toggle"
    role="switch"
    aria-checked={config[key]}
    onclick={() => (config[key] = !config[key])}
  >
    <span>{label}</span>
    <span class="switch" class:on={config[key]}><i></i></span>
  </button>
{/snippet}

<!-- фон всего окна: Originkit Ribbon Glow -->
<div class="bg-glow">
  <RibbonGlow style="position:absolute;inset:0" />
</div>

<div class="shell">
  <header>
    <div class="brand">
      <span class="logo">R1</span>
      <div>
        <h1>OpenShark R1</h1>
        <p>Attack Shark · 2.4G / USB</p>
      </div>
    </div>
    <button
      class="pill"
      onclick={refreshBattery}
      title={batteryKind === "off"
        ? `${batteryText} — нажмите, чтобы обновить.`
        : "Обновить заряд"}
    >
      <span class="dot" data-kind={batteryKind}></span>
      {batteryText}
    </button>
  </header>

  <div class="grid">
    <!-- ЛЕВАЯ КОЛОНКА: опрос и DPI -->
    <aside class="panel">
      <h2>Частота опроса</h2>
      <div class="seg">
        {@render seg("125", 125)}
        {@render seg("250", 250)}
        {@render seg("500", 500)}
        {@render seg("1000", 1000)}
      </div>
      <span class="unit">Гц</span>

      <h2>Ступени DPI</h2>
      <div class="stages">
        {#each config.dpis as dpi, i (i)}
          <!-- строка целиком выбирает ступень (клик-зона = вся строка);
               клавиатура ходит через маркер ниже -->
          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
          <div
            class="stage"
            class:picked={config.active_dpi === i + 1}
            onclick={() => (config.active_dpi = i + 1)}
          >
            <button
              class="marker"
              title="Сделать активной"
              aria-label={`Активировать ступень ${i + 1}`}
              aria-pressed={config.active_dpi === i + 1}
              onclick={() => (config.active_dpi = i + 1)}
            >
              {#if config.active_dpi === i + 1}
                <span class="check">✓</span>
              {/if}
            </button>
            <span class="n">ступень {i + 1}</span>
            <input
              type="number"
              min="100"
              max="18000"
              step="100"
              value={dpi}
              aria-label={`Значение DPI ступени ${i + 1}`}
              onclick={(e) => e.stopPropagation()}
              onchange={(e) => {
                const v = Number((e.target as HTMLInputElement).value);
                config.dpis[i] = Math.max(
                  100,
                  Math.min(18000, Math.round(v / 100) * 100),
                );
              }}
            />
          </div>
        {/each}
      </div>
      <p class="hint">Нажмите ✓, чтобы выбрать активную ступень.</p>
    </aside>

    <!-- ЦЕНТР: 3D-мышь временно убрана (MouseViewer) -->
    <section class="stage-view">
      <div class="hud">
        <span class="hud-label">Активная ступень</span>
        <span class="hud-value">{activeDpi}<em>DPI</em></span>
        <span class="hud-sub">{config.polling_rate} Гц · ступень {config.active_dpi}</span>
      </div>
    </section>

    <!-- ПРАВАЯ КОЛОНКА: таймеры и кнопки -->
    <aside class="panel">
      <h2>Таймеры</h2>

      <label class="slider">
        <span>Сон<i>{config.sleep_time} с</i></span>
        <input
          type="range"
          min="0.5"
          max="30"
          step="0.5"
          bind:value={config.sleep_time}
        />
      </label>

      <label class="slider">
        <span>Глубокий сон<i>{config.deep_sleep_time} с</i></span>
        <input
          type="range"
          min="1"
          max="60"
          step="1"
          bind:value={config.deep_sleep_time}
        />
      </label>

      <label class="slider">
        <span>Отклик кнопки<i>{config.key_response_time} мс</i></span>
        <input
          type="range"
          min="4"
          max="50"
          step="2"
          bind:value={config.key_response_time}
        />
      </label>

      <h2>Датчики</h2>
      {@render toggle("ripple_control", "Ripple control")}
      {@render toggle("angle_snap", "Angle snap")}

      <div class="danger">
        <h2>Кнопки</h2>
        <p>
          Клики перестали работать: скорее всего таблица переназначения в
          мыши сбилась. Восстановление вернёт заводские функции.
        </p>
        <button class="warn" onclick={resetButtons} disabled={status.kind === "busy"}>
          Восстановить кнопки
        </button>
      </div>
    </aside>
  </div>

  <footer>
    {#if status.kind === "err"}
      <span class="banner">{status.text}</span>
    {:else}
      <span class="status" data-kind={status.kind}>{status.text}</span>
    {/if}
    <div class="actions">
      <button class="ghost" onclick={resetDefaults}>По умолчанию</button>
      <button class="primary" onclick={apply} disabled={status.kind === "busy"}>
        Применить
      </button>
    </div>
  </footer>
</div>

<style>
  :global(:root) {
    color-scheme: dark;
    /* Ventura: SF на macOS, системный шрифт дальше по цепочке */
    font-family: -apple-system, "SF Pro Text", system-ui, sans-serif;
    font-size: 13px;
    color: #f5f5f7;
    --accent: #0a84ff;
    --sep: rgba(255, 255, 255, 0.08);
    /* 0.85: худший случай (яркая лента Glow за текстом) даёт ≥5:1 */
    --glass: rgba(30, 30, 34, 0.85);
    --txt-2: #b0b0b5;
    --txt-3: #a6a6ab;
    --focus: #409cff;
    /* синий под белый текст: 5.3:1 (старый #0a84ff давал 3.7:1) */
    --accent-solid: #0069d9;
  }

  :global(*) {
    box-sizing: border-box;
  }

  :global(html),
  :global(body) {
    margin: 0;
    height: 100%;
    overflow: hidden;
    background: #1c1c1e;
  }

  :global(button) {
    font: inherit;
    color: inherit;
    cursor: pointer;
  }

  :global(input) {
    font: inherit;
  }

  /* фокус виден всегда; поверх Glow светлый синий даёт ≥3:1 */
  :global(:focus-visible) {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }

  /* ---- фон всего окна (Ribbon Glow) */
  .bg-glow {
    position: fixed;
    inset: 0;
    z-index: 0;
  }

  .shell {
    position: relative;
    z-index: 1;
    height: 100vh;
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 14px;
    background: transparent;
  }

  header {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 2px 4px;
  }

  /* заголовок лежит прямо на Glow: подложка-скрим, иначе белый текст
     теряет контраст на светлой части ленты (R-25) */
  header::before {
    content: "";
    position: absolute;
    top: -16px;
    left: -18px;
    right: -18px;
    bottom: -14px;
    background: linear-gradient(
      180deg,
      rgba(11, 10, 16, 0.92) 0%,
      rgba(11, 10, 16, 0.9) 65%,
      rgba(11, 10, 16, 0) 100%
    );
    z-index: -1;
    pointer-events: none;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 11px;
  }

  .logo {
    width: 34px;
    height: 34px;
    display: grid;
    place-items: center;
    border-radius: 9px;
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.4px;
    color: #fff;
    /* глубокий синий: белая надпись R1 держит 4.9:1 */
    background: linear-gradient(160deg, #0a6fdc, #0060d2 60%, #0047a8);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.35);
  }

  h1 {
    margin: 0;
    font-size: 15px;
    font-weight: 650;
    letter-spacing: -0.2px;
  }

  .brand p {
    margin: 1px 0 0;
    font-size: 11px;
    color: var(--txt-3);
  }

  .pill {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 6px 13px;
    border-radius: 999px;
    /* плотный фон: чип стоит прямо на Glow, полупрозрачность роняла контраст */
    border: 1px solid rgba(255, 255, 255, 0.35);
    background: rgba(28, 28, 32, 0.9);
    color: #f5f5f7;
    font-variant-numeric: tabular-nums;
    transition: background 0.15s;
  }

  .pill:hover {
    background: rgba(44, 44, 48, 0.94);
  }

  /* точка = реальное состояние, цвет дублируется текстом */
  .dot {
    width: 7px;
    height: 7px;
    flex: none;
    border-radius: 50%;
    background: #8e8e93;
  }

  .dot[data-kind="load"] {
    background: #8e8e93;
  }

  .dot[data-kind="cable"] {
    background: var(--accent);
  }

  .dot[data-kind="ok"] {
    background: #30d158;
  }

  .dot[data-kind="low"] {
    background: #ff9f0a;
  }

  .dot[data-kind="off"] {
    background: #ff453a;
  }

  .grid {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 272px minmax(0, 1fr) 272px;
    gap: 12px;
  }

  .panel {
    min-height: 0;
    overflow-y: auto;
    padding: 15px;
    border-radius: 16px;
    border: 1px solid var(--sep);
    background: var(--glass);
    /* blur только здесь: 2 из дозволенных 1-2, текст ложится на ленту Glow */
    backdrop-filter: blur(28px) saturate(180%);
    -webkit-backdrop-filter: blur(28px) saturate(180%);
    box-shadow:
      0 14px 40px rgba(0, 0, 0, 0.34),
      inset 0 1px 0 rgba(255, 255, 255, 0.06);
  }

  .panel::-webkit-scrollbar {
    width: 7px;
  }

  .panel::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.16);
    border-radius: 4px;
  }

  h2 {
    margin: 0 0 9px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.5px;
    text-transform: uppercase;
    color: var(--txt-2);
  }

  h2:not(:first-child) {
    margin-top: 20px;
  }

  .unit {
    display: block;
    margin-top: 6px;
    font-size: 11px;
    color: var(--txt-3);
  }

  .seg {
    display: flex;
    gap: 3px;
    padding: 3px;
    border-radius: 10px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid var(--sep);
  }

  .seg button {
    flex: 1;
    padding: 6px 0;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: #c7c7cc;
    font-size: 12.5px;
    transition:
      background 0.15s,
      color 0.15s;
  }

  .seg button:hover:not(.on) {
    background: rgba(255, 255, 255, 0.07);
    color: #fff;
  }

  .seg button.on {
    background: linear-gradient(180deg, #0069d9, #0055c2);
    color: #fff;
    font-weight: 600;
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.3);
  }

  .stages {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .stage {
    position: relative;
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 6px 9px;
    border-radius: 10px;
    border: 1px solid var(--sep);
    background: rgba(255, 255, 255, 0.045);
    transition:
      background 0.15s,
      border-color 0.15s;
  }

  .stage:hover {
    background: rgba(255, 255, 255, 0.08);
  }

  .stage.picked {
    border-color: rgba(64, 156, 255, 0.6);
    background: rgba(10, 132, 255, 0.16);
  }

  /* кольцо 0.5: граница маркера держит 3:1 к фону строки */
  .marker {
    width: 17px;
    height: 17px;
    flex: none;
    padding: 0;
    position: relative;
    display: grid;
    place-items: center;
    border-radius: 50%;
    border: 1.5px solid rgba(255, 255, 255, 0.5);
    background: transparent;
    font-size: 10px;
    line-height: 1;
    color: #fff;
  }

  /* зона клика 33px: кольцо визуально мелкое, тап-зона должна быть удобной */
  .marker::after {
    content: "";
    position: absolute;
    inset: -8px;
  }

  /* активный маркер: белый кружок, синяя галочка (5.3:1) */
  .stage.picked .marker {
    border-color: #fff;
    background: #fff;
    color: var(--accent-solid);
  }

  .n {
    flex: 1;
    font-size: 12.5px;
    color: #c7c7cc;
  }

  .stage.picked .n {
    color: #fff;
  }

  .stage input {
    width: 74px;
    padding: 4px 7px;
    text-align: right;
    font-variant-numeric: tabular-nums;
    color: #f5f5f7;
    background: rgba(0, 0, 0, 0.34);
    /* 0.45: рамка поля сама себя объясняет, 3.8:1 к панели */
    border: 1px solid rgba(255, 255, 255, 0.45);
    border-radius: 7px;
  }

  .stage input:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px rgba(10, 132, 255, 0.25);
  }

  .hint {
    margin: 9px 0 0;
    font-size: 11px;
    line-height: 1.45;
    color: var(--txt-3);
  }

  /* ---- центральная сцена */
  .stage-view {
    position: relative;
    min-height: 0;
    border-radius: 18px;
    overflow: hidden;
    border: 1px solid var(--sep);
    background: transparent;
    box-shadow:
      0 20px 60px rgba(0, 0, 0, 0.4),
      inset 0 1px 0 rgba(255, 255, 255, 0.07);
  }

  .hud {
    position: absolute;
    left: 50%;
    bottom: 18px;
    transform: translateX(-50%);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1px;
    padding: 11px 30px;
    /* abs-блок без ширины жмётся до половины сцены и переносил подписи */
    white-space: nowrap;
    border-radius: 15px;
    border: 1px solid var(--sep);
    /* плотный фон вместо blur: кап стекла отдан двум панелям */
    background: rgba(28, 28, 32, 0.92);
    pointer-events: none;
  }

  .hud-label {
    font-size: 10px;
    letter-spacing: 0.7px;
    text-transform: uppercase;
    color: var(--txt-2);
  }

  .hud-value {
    font-size: 34px;
    font-weight: 700;
    letter-spacing: -1.2px;
    line-height: 1.1;
    background: linear-gradient(180deg, #ffffff, #b9c4d6);
    -webkit-background-clip: text;
    background-clip: text;
    color: transparent;
  }

  .hud-value em {
    font-style: normal;
    font-size: 14px;
    font-weight: 600;
    letter-spacing: 0;
    margin-left: 5px;
    /* светлый синий: 5.5:1 на плотном фоне HUD */
    color: #4da3ff;
    -webkit-text-fill-color: #4da3ff;
  }

  .hud-sub {
    font-size: 11px;
    color: var(--txt-3);
    font-variant-numeric: tabular-nums;
  }

  /* ---- слайдеры */
  .slider {
    display: block;
    margin-bottom: 15px;
  }

  .slider > span {
    display: flex;
    justify-content: space-between;
    margin-bottom: 7px;
    font-size: 12.5px;
    color: #c7c7cc;
  }

  .slider i {
    font-style: normal;
    color: #f5f5f7;
    font-variant-numeric: tabular-nums;
  }

  .slider input {
    -webkit-appearance: none;
    appearance: none;
    width: 100%;
    height: 4px;
    border-radius: 3px;
    background: rgba(255, 255, 255, 0.22);
  }

  .slider input::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 17px;
    height: 17px;
    border-radius: 50%;
    background: #fff;
    border: 0.5px solid rgba(0, 0, 0, 0.25);
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.45);
    cursor: grab;
  }

  .slider input::-webkit-slider-thumb:active {
    cursor: grabbing;
    background: var(--accent-solid);
  }

  /* ---- тумблеры */
  .row {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 9px 10px;
    margin-bottom: 6px;
    border-radius: 10px;
    border: 1px solid var(--sep);
    background: rgba(255, 255, 255, 0.045);
    font-size: 12.5px;
    color: #c7c7cc;
    text-align: left;
    transition: background 0.15s;
  }

  .row:hover {
    background: rgba(255, 255, 255, 0.08);
  }

  .switch {
    position: relative;
    flex: none;
    width: 37px;
    height: 22px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.16);
    /* внутреннее кольцо: граница выключенного тумблера к фону строки 3.2:1 */
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.3);
    transition: background 0.2s;
  }

  .switch i {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 2px 5px rgba(0, 0, 0, 0.4);
    transition: transform 0.2s;
  }

  .switch.on {
    background: #30d158;
    box-shadow: none;
  }

  .switch.on i {
    transform: translateX(15px);
  }

  /* ---- опасная зона */
  .danger {
    margin-top: 20px;
    padding: 12px;
    border-radius: 12px;
    border: 1px solid rgba(255, 159, 10, 0.28);
    background: rgba(255, 159, 10, 0.08);
  }

  .danger h2 {
    margin-top: 0;
    color: #ffb86c;
  }

  .danger p {
    margin: 0 0 10px;
    font-size: 11.5px;
    line-height: 1.5;
    color: #b8b3ac;
  }

  .warn {
    width: 100%;
    padding: 8px 12px;
    border-radius: 9px;
    border: 1px solid rgba(255, 159, 10, 0.5);
    background: linear-gradient(180deg, rgba(255, 159, 10, 0.3), rgba(255, 159, 10, 0.16));
    color: #ffd9a8;
    font-weight: 600;
    font-size: 12.5px;
    transition: filter 0.15s;
  }

  .warn:hover:not(:disabled) {
    filter: brightness(1.18);
  }

  .warn:disabled {
    opacity: 0.55;
  }

  /* ---- футер */
  footer {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 11px 14px;
    border-radius: 15px;
    border: 1px solid var(--sep);
    /* плотно, без blur (кап стекла): статусы держат 5:1+ */
    background: rgba(30, 30, 34, 0.95);
    box-shadow:
      0 14px 40px rgba(0, 0, 0, 0.34),
      inset 0 1px 0 rgba(255, 255, 255, 0.06);
  }

  .status {
    font-size: 12.5px;
    color: var(--txt-3);
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .status[data-kind="ok"] {
    color: #30d158;
  }

  .status[data-kind="busy"] {
    color: #ffd60a;
  }

  .banner {
    font-size: 12.5px;
    color: #ff8a8a;
    word-break: break-word;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .actions {
    display: flex;
    gap: 8px;
    margin-left: auto;
    flex: none;
  }

  .actions button {
    padding: 8px 15px;
    border-radius: 9px;
    border: 1px solid var(--sep);
    font-size: 12.5px;
    font-weight: 550;
    transition:
      background 0.15s,
      filter 0.15s;
  }

  .ghost {
    background: rgba(255, 255, 255, 0.08);
    /* 0.4: граница кнопки к футеру 3:1 (фон кнопки почти неотличим) */
    border-color: rgba(255, 255, 255, 0.4);
    color: #d1d1d6;
  }

  .ghost:hover {
    background: rgba(255, 255, 255, 0.14);
  }

  .primary {
    background: linear-gradient(180deg, #0069d9, #0055c2);
    border-color: rgba(255, 255, 255, 0.18);
    color: #fff;
    font-weight: 650;
  }

  .primary:hover:not(:disabled) {
    filter: brightness(1.1);
  }

  .actions button:disabled {
    opacity: 0.55;
  }

  /* Низкая ширина: центр перестаёт вмещать HUD (брейкпоинт там, где
     контент реально ломается: 272*2 + отступы + 220 под HUD = 820).
     Три колонки складываются в одну, футер с «Применить» залипает снизу. */
  @media (max-width: 820px) {
    .shell {
      overflow-y: auto;
    }

    .grid {
      grid-template-columns: minmax(0, 1fr);
      grid-auto-rows: min-content;
    }

    .stage-view {
      order: -1;
      min-height: 240px;
    }

    .panel {
      overflow: visible;
    }

    footer {
      position: sticky;
      bottom: 0;
      z-index: 2;
    }
  }
</style>
