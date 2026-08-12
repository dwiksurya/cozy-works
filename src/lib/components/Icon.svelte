<script>
  export let name = "check";
  export let size = 18;
  export let stroke = 2;

  // Pixelarticons paths (24x24 viewBox, stroke-based)
  const paths = {
    home: "M4 21V9l8-6 8 6v12h-6v-7h-4v7H4z",
    check: "M4 12l5 5L20 6",
    "check-double": "M2 12l5 5L18 6M12 13l3 3 7-8",
    note: "M4 4h16v16H4zM4 9h16M4 14h16M9 4v16",
    doc: "M6 2h9l5 5v15H6V2zM14 2v6h6M9 12h6M9 16h6",
    music: "M9 18V5l10-2v13M9 18a2 2 0 11-4 0 2 2 0 014 0zm10-2a2 2 0 11-4 0 2 2 0 014 0z",
    terminal: "M4 5h16v14H4V5zm3 4l4 3-4 3M12 15h6",
    settings: "M12 8a4 4 0 100 8 4 4 0 000-8zm7 4h1-1zm-16 0h-1 1zm12.5-4.5l.7-.7-.7.7zM5.8 17.2l-.7.7.7-.7zM19.5 12h1-1zm-15 0h-1 1zm9.9 4.5l.7.7-.7-.7zM7 6.8l-.7-.7.7.7z",
    clock: "M12 21a9 9 0 100-18 9 9 0 000 18zm0-14v5l3 2",
    list: "M4 6h1M9 6h11M4 12h1M9 12h11M4 18h1M9 18h11",
    play: "M7 4l13 8-13 8V4z",
    pause: "M7 4h4v16H7zM13 4h4v16h-4z",
    "skip-back": "M6 5v14M19 5l-9 7 9 7V5z",
    "skip-forward": "M18 5v14M5 5l9 7-9 7V5z",
    repeat: "M17 2l3 3-3 3M4 11V8a3 3 0 013-3h13M7 22l-3-3 3-3M20 13v3a3 3 0 01-3 3H4",
    shuffle: "M3 6h3l12 12h3M3 18h3l3-3M17 6h4M15 9l6 6",
    volume: "M4 9v6h4l6 5V4L8 9H4zm11 2a3 3 0 010 4M15 7a6 6 0 010 10",
    "volume-2": "M4 9v6h4l6 5V4L8 9H4zm11 2a3 3 0 010 4M15 7a6 6 0 010 10",
    send: "M3 20L22 12 3 4v6l13 2-13 2v6z",
    user: "M12 12a4 4 0 100-8 4 4 0 000 8zm-8 9a8 8 0 0116 0",
    star: "M12 3l2.7 5.8 6.3.8-4.6 4.3 1.2 6.1L12 17.5 6.4 20l1.2-6.1L3 9.6l6.3-.8L12 3z",
    pin: "M12 3l5 5-2 4v6l-3 3-3-3v-6L7 8l5-5z",
    search: "M11 19a8 8 0 100-16 8 8 0 000 16zm10 2l-4.3-4.3",
    plus: "M12 5v14M5 12h14",
    edit: "M4 20h4L20 8l-4-4L4 16v4zM13.5 6.5l4 4",
    trash: "M5 7h14M9 7V4h6v3M7 7l1 13h8l1-13M10 11v6M14 11v6",
    close: "M5 5l14 14M19 5L5 19",
    bell: "M12 3a5 5 0 00-5 5v4l-2 4h14l-2-4V8a5 5 0 00-5-5zm-3 14a3 3 0 006 0",
    calendar: "M4 5h16v16H4V5zm3-3v4M17 2v4M4 10h16",
    tag: "M4 4h8l8 8-8 8-8-8V4zm3 3a1 1 0 100 2 1 1 0 000-2z",
    flag: "M5 21V4h11l-2 4 2 4H5",
    moon: "M20 14A8 8 0 1110 4a7 7 0 0010 10z",
    sun: "M12 7a5 5 0 100 10 5 5 0 000-10zm0-4v2M12 19v2M5 5l1.5 1.5M17.5 17.5L19 19M3 12h2M19 12h2M5 19l1.5-1.5M17.5 6.5L19 5",
    cloud: "M7 19a4 4 0 010-8 6 6 0 0111.4-1.5A4.5 4.5 0 0118 19H7z",
    "cloud-rain": "M7 19a4 4 0 010-8 6 6 0 0111.4-1.5A4.5 4.5 0 0118 19H7zM8 20l-1 2M12 20l-1 2M16 20l-1 2",
    city: "M4 21V9l6 4V9l6 4V5h4v16H4zm3-8h2M9 17h2M15 13h2M17 17h2",
    paw: "M12 13c-1.5 0-3-2-3-4s1.5-4 3-4 3 2 3 4-1.5 4-3 4zM6 14c-1.7 0-3-2-3-4s1.3-3.5 3-3.5S9 8.5 9 10s-1.3 4-3 4zm12 0c-1.7 0-3-2-3-4s1.3-3.5 3-3.5S21 8.5 21 10s-1.3 4-3 4zM4 16c2-1 3 1 5 1s3-2 5-2 3 2 5 2 3-2 5-1c-1 4-4 6-10 6S5 20 4 16z",
    heart: "M12 20s-7-4.5-9.5-9C1 7.5 3 4 6.5 4 9 4 12 6.5 12 6.5S15 4 17.5 4C21 4 23 7.5 21.5 11c-2.5 4.5-9.5 9-9.5 9z",
    mic: "M12 15a4 4 0 004-4V6a4 4 0 10-8 0v5a4 4 0 004 4zm-7-4a7 7 0 0014 0M12 18v4",
    "arrow-left": "M19 12H5m7-7l-7 7 7 7",
    "arrow-right": "M5 12h14m-7-7l7 7-7 7",
    "chevron-down": "M6 9l6 6 6-6",
    backspace: "M4 12l5-6h11v12H9l-5-6zm4-2l2-2M8 14l2-2M14 14l-2-2 2-2 2 2-2 2",
    "tab-plus": "M4 4h7v7H4V4zm9 0h7v7h-7V4zM4 13h7v7H4v-7zm13 0h3M17 16h3M17 13v6M20.5 16H17",
    "layout-sidebar": "M4 4h16v16H4V4zm7 0v16",
    spark: "M12 3l1.5 4.5L18 9l-4.5 1.5L12 15l-1.5-4.5L6 9l4.5-1.5L12 3zM19 14l.8 2.2L22 17l-2.2.8L19 20l-.8-2.2L16 17l2.2-.8L19 14z",
    "cloud-sun": "M7 19a4 4 0 010-8 6 6 0 0111.4-1.5A4.5 4.5 0 0118 19H7zM13 4v2M6 5l1.5 1.5M3 11h2",
    "sun-cloud": "M13 4v2M6 5l1.5 1.5M3 11h2M7 19a4 4 0 010-8 6 6 0 0111.4-1.5A4.5 4.5 0 0118 19H7z",
    coffee: "M4 9h13v5a5 5 0 01-5 5H9a5 5 0 01-5-5V9zm13 2h2a2 2 0 010 4h-2M7 4v3M11 4v3M15 4v3",
    "paint-bucket": "M6 9h12l-6-6-6 6zm-2 1l14 9 2-2-12-9M5 20h14",
    "checkbox-on": "M4 4h16v16H4V4zm3 8l3 3 7-7",
    "checkbox-off": "M4 4h16v16H4V4z",
    "align-left": "M4 6h16M4 10h10M4 14h10M4 18h16",
    "dots-h": "M5 12a1 1 0 100 2 1 1 0 000-2zm7 0a1 1 0 100 2 1 1 0 000-2zm7 0a1 1 0 100 2 1 1 0 000-2z",
    info: "M12 21a9 9 0 100-18 9 9 0 000 18zm0-9v5M12 8h.01",
    "alert-triangle": "M12 3l10 18H2L12 3zm0 6v5M12 16.5v.01",
  };

  // Paths without stroke (filled icons)
  const filled = {
    check: "M9 16.2l-3.5-3.5-1.4 1.4L9 19 21 7l-1.4-1.4L9 16.2z",
  };
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width={stroke}
  stroke-linecap="round"
  stroke-linejoin="round"
  style="flex-shrink: 0;"
>
  {#if paths[name]}
    <path d={paths[name]} />
  {:else if filled[name]}
    <path d={filled[name]} fill="currentColor" stroke="none" />
  {:else}
    <circle cx="12" cy="12" r="9" stroke-dasharray="3 3" />
  {/if}
</svg>
