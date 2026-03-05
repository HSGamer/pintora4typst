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
      index_default.setConfig(n);
      
      globalThis._pintoraLastWarning = "";
      
      csrc.innerText = e;
      rslt.innerHTML = "";
      pintoraStandalone.renderContentOf(csrc, { resultContainer: rslt });
      if ("" === rslt.innerHTML) {
          throw new Error("\n Rendering Error: " + (globalThis._pintoraLastWarning || "Unknown issue format"));
      }
      rslt.firstChild.setAttribute("xmlns", "http://www.w3.org/2000/svg");
      return rslt.innerHTML;
    }
