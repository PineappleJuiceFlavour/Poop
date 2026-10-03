using System; using System.IO; using System.Linq; using System.Reflection;
var dir = @"C:\Users\Admin\Downloads\JCtools\tools\RBMesh\";
var a = Assembly.LoadFrom(dir + "JustCause.Archives.Core.dll");
var aaf = a.GetType("JustCause.Archives.Core.AafDecompressor"); var sarcT = a.GetType("JustCause.Archives.Core.SarcArchive");
var data = File.ReadAllBytes(args[0]);
if ((bool)aaf.GetMethod("IsAaf").Invoke(null, new object[]{data})) data = (byte[])aaf.GetMethod("Decompress").Invoke(null, new object[]{data});
var s = sarcT.GetMethod("Parse").Invoke(null, new object[]{data});
foreach (var e in (System.Collections.IEnumerable)sarcT.GetProperty("Entries").GetValue(s)) {
  var t = e.GetType(); var name = (string)t.GetField("Name").GetValue(e);
  Console.WriteLine(name + "  " + t.GetField("Size").GetValue(e) + (((bool)t.GetProperty("IsReference").GetValue(e)) ? "  (ref)" : ""));
  if (args.Length > 1 && name.EndsWith(args[1])) File.WriteAllBytes(Path.Combine(args[2], Path.GetFileName(name)), (byte[])sarcT.GetMethod("Extract").Invoke(s, new object[]{name}));
}
