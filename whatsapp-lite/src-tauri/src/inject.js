(function() {
  console.log("WhatsApp Tauri: Initialized.");

  const style = document.createElement('style');
  style.innerHTML = `
    /* Hide desktop app download banners */
    ._3X_7s, ._106uX, ._2S6p_ { display: none !important; }
    /* Ensure the app fills the window */
    #app, .app-wrapper { height: 100% !important; width: 100% !important; position: absolute; top: 0; left: 0; }
  `;
  document.head.appendChild(style);
})();
