// jc3export: Just Cause 3 models (.ee/.bl/.nl archives or loose .rbm) -> glTF + PNG, from the user's own unpacked
// game files, using the JustCause.*.Core libraries that ship with RBMesh.
// Usage: jc3export <files root> <out dir> <input path or folder> [more inputs...]
using System.IO.Compression;
using JustCause.Archives.Core;
using JustCause.Models.Core;
using JustCause.Textures.Core;

if (args.Length < 3)
{
    Console.WriteLine("usage: jc3export <files root> <out dir> <input...>");
    return 1;
}
var root = Path.GetFullPath(args[0]);
var outDir = Path.GetFullPath(args[1]);
Directory.CreateDirectory(outDir);
var resolver = new SiblingFileResolver();
var texCache = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase); // archive path -> png file name
var manifest = new List<string> { "model,source,blocks,triangles,textures" };
int ok = 0, failed = 0;

foreach (var input in args.Skip(2))
{
    var files = Directory.Exists(input)
        ? Directory.EnumerateFiles(input, "*", SearchOption.AllDirectories).Where(f => f.EndsWith(".ee") || f.EndsWith(".bl") || f.EndsWith(".nl") || f.EndsWith(".rbm"))
        : new[] { input };
    foreach (var f in files)
    {
        try { Export(f); }
        catch (Exception e) { failed++; Console.WriteLine($"FAIL {f}: {e.Message}"); }
    }
}
File.WriteAllLines(Path.Combine(outDir, "manifest.csv"), manifest);
Console.WriteLine($"done: {ok} models, {texCache.Count} textures, {failed} failed");
return 0;

void Export(string path)
{
    var data = File.ReadAllBytes(path);
    if (path.EndsWith(".rbm"))
    {
        ExportModel(Path.GetFileNameWithoutExtension(path), path, data, null);
        return;
    }
    if (AafDecompressor.IsAaf(data)) data = AafDecompressor.Decompress(data);
    var sarc = SarcArchive.Parse(data);
    var baseName = Path.GetFileNameWithoutExtension(path);
    foreach (var e in sarc.Entries.Where(e => e.Name.EndsWith(".rbm", StringComparison.OrdinalIgnoreCase)))
    {
        byte[] bytes = e.IsReference ? ReadRoot(e.Name) : sarc.Extract(e.Name);
        if (bytes == null) { Console.WriteLine($"  missing {e.Name}"); continue; }
        // _lod1 is the full-detail mesh; skip the lower LODs
        var stem = Path.GetFileNameWithoutExtension(e.Name);
        if (System.Text.RegularExpressions.Regex.IsMatch(stem, "_lod[2-9]$")) continue;
        ExportModel(baseName + "__" + stem, path + ":" + e.Name, bytes, sarc);
    }
}

byte[] ReadRoot(string archivePath)
{
    var p = Path.Combine(root, archivePath.Replace('/', Path.DirectorySeparatorChar));
    return File.Exists(p) ? File.ReadAllBytes(p) : null;
}

void ExportModel(string name, string source, byte[] rbm, SarcArchive sarc)
{
    var model = RbmParser.Parse(rbm);
    var mats = new List<GltfExporter.BlockMaterial>();
    int texCount = 0;
    foreach (var b in model.Blocks)
    {
        var m = new GltfExporter.BlockMaterial { Name = b.TypeName };
        m.BaseColorUri = Texture(b.DiffuseTexture, sarc);
        m.NormalUri = Texture(b.NormalTexture, sarc);
        if (m.BaseColorUri != null) texCount++;
        mats.Add(m);
    }
    var safe = string.Concat(name.Select(c => char.IsLetterOrDigit(c) || c == '_' || c == '-' ? c : '_'));
    WriteJcm(Path.Combine(outDir, safe + ".jcm"), model, mats);
    var gltf = GltfExporter.Export(model, mats, safe + ".bin", out var bin);
    File.WriteAllText(Path.Combine(outDir, safe + ".gltf"), gltf);
    File.WriteAllBytes(Path.Combine(outDir, safe + ".bin"), bin);
    manifest.Add($"{safe},{source},{model.Blocks.Count},{model.TotalTriangles},{texCount}");
    ok++;
    Console.WriteLine($"OK {safe}  ({model.Blocks.Count} blocks, {model.TotalTriangles} tris, {texCount} textured)");
    foreach (var w in model.Warnings.Take(3)) Console.WriteLine("  warn: " + w);
}

string Texture(string archivePath, SarcArchive sarc)
{
    if (string.IsNullOrEmpty(archivePath)) return null;
    if (texCache.TryGetValue(archivePath, out var cached)) return cached;
    string png = null;
    try
    {
        var disk = Path.Combine(root, archivePath.Replace('/', Path.DirectorySeparatorChar));
        if (!File.Exists(disk) && sarc != null && sarc.Entries.Any(e => !e.IsReference && e.Name.Equals(archivePath, StringComparison.OrdinalIgnoreCase)))
        {
            // texture packed inside the archive itself: stage it next to its .hmddsc sibling if there is one
            Directory.CreateDirectory(Path.GetDirectoryName(disk));
            File.WriteAllBytes(disk + ".tmp", sarc.Extract(archivePath));
            File.Move(disk + ".tmp", disk);
        }
        if (File.Exists(disk))
        {
            var bundle = AvtxBundle.OpenFile(disk, resolver);
            var img = bundle.SubTextures[0].DecodeLargest();
            png = Path.GetFileNameWithoutExtension(archivePath) + ".png";
            WritePng(Path.Combine(outDir, png), img.Width, img.Height, img.Pixels);
            WriteRgba(Path.Combine(outDir, Path.ChangeExtension(png, ".rgba")), img.Width, img.Height, img.Pixels);
        }
    }
    catch (Exception e) { Console.WriteLine($"  texture {archivePath}: {e.Message}"); png = null; }
    texCache[archivePath] = png;
    return png;
}

// .jcm, read by RicoKit's gear renderer in GTA V. Little-endian:
//   "JCM1", u32 blockCount, then per block: u16 nameLen + ASCII diffuse texture file (.rgba, may be empty),
//   u32 vertexCount, vertexCount * (pos xyz, normal xyz, uv) floats, u32 indexCount, indexCount * u32.
// Positions stay in JC3's model space (metres, Y up).
static void WriteJcm(string path, RbmModel model, List<GltfExporter.BlockMaterial> mats)
{
    using var w = new BinaryWriter(File.Create(path));
    w.Write("JCM1"u8.ToArray());
    var blocks = model.Blocks.Where(b => b.Decoded && b.Vertices.Count > 0).ToList();
    w.Write((uint)blocks.Count);
    foreach (var b in blocks)
    {
        var tex = mats[model.Blocks.IndexOf(b)]?.BaseColorUri;
        var name = tex == null ? "" : Path.ChangeExtension(tex, ".rgba");
        w.Write((ushort)name.Length);
        w.Write(System.Text.Encoding.ASCII.GetBytes(name));
        w.Write((uint)b.Vertices.Count);
        foreach (var v in b.Vertices)
        {
            w.Write(v.Position.X); w.Write(v.Position.Y); w.Write(v.Position.Z);
            w.Write(v.Normal.X); w.Write(v.Normal.Y); w.Write(v.Normal.Z);
            w.Write(v.Uv0.U); w.Write(v.Uv0.V);
        }
        var idx = b.Indices.Cast<object>().Select(Convert.ToUInt32).ToList();
        w.Write((uint)idx.Count);
        foreach (var i in idx) w.Write(i);
    }
}

// .rgba: u32 width, u32 height, then RGBA8 rows; halved until it fits 1024x1024 to keep GPU memory small
static void WriteRgba(string path, int w, int h, byte[] px)
{
    while (w > 1024 || h > 1024)
    {
        int nw = Math.Max(1, w / 2), nh = Math.Max(1, h / 2);
        var half = new byte[nw * nh * 4];
        for (int y = 0; y < nh; y++)
            for (int x = 0; x < nw; x++)
                for (int c = 0; c < 4; c++)
                {
                    int s = px[((2 * y) * w + 2 * x) * 4 + c] + px[((2 * y) * w + Math.Min(2 * x + 1, w - 1)) * 4 + c]
                          + px[(Math.Min(2 * y + 1, h - 1) * w + 2 * x) * 4 + c] + px[(Math.Min(2 * y + 1, h - 1) * w + Math.Min(2 * x + 1, w - 1)) * 4 + c];
                    half[(y * nw + x) * 4 + c] = (byte)(s / 4);
                }
        px = half; w = nw; h = nh;
    }
    using var f = new BinaryWriter(File.Create(path));
    f.Write((uint)w); f.Write((uint)h); f.Write(px, 0, w * h * 4);
}

static void WritePng(string path, int w, int h, byte[] rgba)
{
    using var fs = File.Create(path);
    fs.Write(new byte[] { 137, 80, 78, 71, 13, 10, 26, 10 });
    var ihdr = new byte[13];
    BE(ihdr, 0, (uint)w); BE(ihdr, 4, (uint)h);
    ihdr[8] = 8; ihdr[9] = 6;
    Chunk(fs, "IHDR", ihdr);
    var raw = new MemoryStream();
    for (int y = 0; y < h; y++) { raw.WriteByte(0); raw.Write(rgba, y * w * 4, w * 4); }
    var z = new MemoryStream();
    using (var zl = new ZLibStream(z, CompressionLevel.Fastest, true)) raw.WriteTo(zl);
    Chunk(fs, "IDAT", z.ToArray());
    Chunk(fs, "IEND", Array.Empty<byte>());
}

static void Chunk(Stream s, string type, byte[] data)
{
    var len = new byte[4]; BE(len, 0, (uint)data.Length); s.Write(len);
    var td = System.Text.Encoding.ASCII.GetBytes(type).Concat(data).ToArray();
    s.Write(td);
    var crc = new byte[4]; BE(crc, 0, Crc(td)); s.Write(crc);
}

static void BE(byte[] b, int o, uint v) { b[o] = (byte)(v >> 24); b[o + 1] = (byte)(v >> 16); b[o + 2] = (byte)(v >> 8); b[o + 3] = (byte)v; }

static uint Crc(byte[] d)
{
    uint c = 0xFFFFFFFF;
    foreach (var x in d) { c ^= x; for (int k = 0; k < 8; k++) c = (c & 1) != 0 ? 0xEDB88320 ^ (c >> 1) : c >> 1; }
    return c ^ 0xFFFFFFFF;
}
