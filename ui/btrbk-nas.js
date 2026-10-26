/* DSM desktop window: a native app window holding the interface. */
SYNO.namespace("SYNO.SDS.BtrbkNas");

SYNO.SDS.BtrbkNas.Instance = Vue.extend({
    template:
        '<v-app-instance class-name="SYNO.SDS.BtrbkNas.Instance">' +
        '<v-app-window ref="appWindow" syno-id="SYNO.SDS.BtrbkNas.Window"' +
        ' width="1180" height="760" :resizable="true">' +
        '<iframe src="/webman/3rdparty/btrbk-nas/index.html" title="btrbk Backups"' +
        ' style="display:block;width:100%;height:100%;border:0"></iframe>' +
        "</v-app-window>" +
        "</v-app-instance>",
});
