class ConsoleStub {
  constructor() {
    this.logHistory   = [];
    this.errorHistory = [];
    this.warnHistory  = [];
  }
  log(...args) { this.logHistory.push(args.join(' ')); }
  error(...args) { this.errorHistory.push(args.join(' ')); }
  warn(...args) { this.warnHistory.push(args.join(' ')); }
}
var console = new ConsoleStub();
