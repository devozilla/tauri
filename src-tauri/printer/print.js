const escpos = require("escpos");
escpos.USB = require("escpos-usb");

module.exports = function printBarcode(data) {
  const device = new escpos.USB();
  const printer = new escpos.Printer(device);

  device.open(() => {
    printer
      .text(data.name)
      .barcode(data.code, "CODE128")
      .cut()
      .close();
  });
};