/* DSM desktop window: a native app window holding the interface. */
SYNO.namespace("SYNO.SDS.BtrbkNas");

/*
 * DSM keeps this loader for the whole desktop session, across package
 * upgrades: the page it opens must not depend on it. index.html is asked
 * for fresh every time (it is small, and names the build of everything
 * else), and the frame takes the theme's background at once, so nothing
 * flashes white while it loads.
 */
SYNO.SDS.BtrbkNas.Instance = Vue.extend({
    template:
        '<v-app-instance class-name="SYNO.SDS.BtrbkNas.Instance">' +
        '<v-app-window ref="appWindow" syno-id="SYNO.SDS.BtrbkNas.Window"' +
        ' width="1180" height="760" :resizable="true">' +
        '<iframe :src="src" title="btrbk Backups" :style="frameStyle"></iframe>' +
        "</v-app-window>" +
        "</v-app-instance>",
    data: function () {
        // the page's choice (same origin), else dark, as the page
        var theme = null;
        try {
            theme = localStorage.getItem("btrbk-nas.theme");
        } catch (e) {}
        return {
            src: "/webman/3rdparty/btrbk-nas/index.html?t=" + Date.now(),
            frameStyle: "display:block;width:100%;height:100%;border:0;background:" + (theme === "light" ? "#e6e7ed" : "#14171c"),
        };
    },
});
