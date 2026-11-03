/* Demo mode only (tools/devserver.py --demo): sets the scene a screenshot
   needs, from the URL. ?shot=settings opens the settings; ?shot=queue queues
   the deletion of the oldest snapshots shown. */
(() => {
  const shot = new URLSearchParams(location.search).get("shot");
  const when = (test, then) => {
    const tick = () => (test() ? then() : setTimeout(tick, 50));
    tick();
  };
  if (shot === "settings") {
    when(() => document.querySelector(".side-item"), () => {
      document.getElementById("open-settings").click();
      setTimeout(() => document.activeElement.blur(), 300);
    });
  }
  if (shot === "queue") {
    when(() => document.querySelector("td.actions .btn.danger"), () => {
      const buttons = [...document.querySelectorAll("td.actions .btn.danger")];
      buttons.slice(-3).forEach((b) => b.click());
      document.querySelector(".tabs, table").scrollIntoView();
    });
  }
})();
