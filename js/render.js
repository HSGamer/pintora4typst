    var document = new Document();
    var csrc = document.createElement("div");
    csrc.dataset=[];
    var rslt = document.createElement("svg");

    csrc.dataset['renderer'];

    globalThis.PintoraRender = function(e, t = "default", A = "Source Code Pro, sans-serif") {
      csrc.dataset.theme = t;
      var n = config;
      n.core.defaultFontFamily = A;
      configApi.setConfig(n);
      runtime_default.setConfig(n);
      console = new ConsoleStub();
      csrc.innerText = e;
      rslt.innerHTML = "";
      pintoraStandalone.renderContentOf(csrc, { resultContainer: rslt });
      if ("" === rslt.innerHTML) throw new Error("\n " + String(console.warnHistory.slice(-1)));
      rslt.firstChild.setAttribute("xmlns", "http://www.w3.org/2000/svg");
      return rslt.innerHTML;
    }
