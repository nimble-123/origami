// Benchmark harness for docs/benchmark-abap-cleaner.md (not built by cargo).
import com.sap.adt.abapcleaner.base.ABAP;
import com.sap.adt.abapcleaner.parser.ParseParams;
import com.sap.adt.abapcleaner.programbase.*;
import com.sap.adt.abapcleaner.rulebase.*;
import java.nio.file.*;
import java.util.*;

// Usage: Bench <default|essential|parse> <file-list> <passes> [per-file-out]
// With per-file-out, the last pass writes one line per file:
// "<microseconds>\t<path>" (used by `cargo xtask bench-matrix`).
public class Bench {
  public static void main(String[] a) throws Exception {
    Program.initialize(null, "");
    String mode = a[0];
    List<String> files = Files.readAllLines(Path.of(a[1]));
    int passes = Integer.parseInt(a[2]);
    List<String> codes = new ArrayList<>();
    for (String f : files) codes.add(Files.readString(Path.of(f)));
    Profile profile = mode.equals("essential") ? Profile.createEssential() : Profile.createDefault();
    StringBuilder perFile = new StringBuilder();
    for (int p = 1; p <= passes; p++) {
      long t0 = System.nanoTime();
      int errors = 0, lines = 0;
      perFile.setLength(0);
      for (int i = 0; i < codes.size(); i++) {
        long f0 = System.nanoTime();
        CleanupParams cp = mode.equals("parse") ? CleanupParams.createForParseOnly()
            : CleanupParams.createForProfile(profile, false);
        Job job = Job.createForSingleCodeDocument(
            ParseParams.createForWholeCode(files.get(i), codes.get(i), ABAP.NEWEST_RELEASE), cp);
        Task t = job.run();
        if (t.getParseError() != null || t.getCleanupError() != null || t.getCheckError() != null) errors++;
        lines += t.getLineCountInCleanupRange();
        perFile.append((System.nanoTime() - f0) / 1_000).append('\t').append(files.get(i)).append('\n');
      }
      long ms = (System.nanoTime() - t0) / 1_000_000;
      System.out.printf("abapcleaner %s pass %d: %d files, %d lines, %d ms, %d with errors%n",
          mode, p, codes.size(), lines, ms, errors);
    }
    if (a.length > 3) Files.writeString(Path.of(a[3]), perFile.toString());
  }
}
