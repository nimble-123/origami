// ABAP Cleaner side of site/abap-cleaner.html (not built by cargo; see build.py).
import com.sap.adt.abapcleaner.base.ABAP;
import com.sap.adt.abapcleaner.parser.ParseParams;
import com.sap.adt.abapcleaner.programbase.*;
import com.sap.adt.abapcleaner.rulebase.*;
import java.nio.file.*;
import java.util.*;

// Usage:
//   Compare profile                      rules of the layout-only profile
//   Compare format <in> <out>            clean one file with that profile
//   Compare format-all <file-list> <dir> clean many files into <dir>/<n>.abap
//                                        (a file ABAP Cleaner fails on is copied)
//   Compare scan <file-list> <out.tsv>   every rule alone over many files:
//       files whose whitespace or letter case changed, files whose other
//       text changed (code or comments)
public class Compare {
  /** Rule groups that only change layout. */
  static final Set<RuleGroupID> LAYOUT_GROUPS = EnumSet.of(RuleGroupID.EMPTY_LINES,
      RuleGroupID.SPACES, RuleGroupID.PRETTY_PRINTER, RuleGroupID.ALIGNMENT,
      RuleGroupID.DDL_SPACES, RuleGroupID.DDL_ALIGNMENT, RuleGroupID.DDL_EMPTY_LINES,
      RuleGroupID.DDL_POSITION, RuleGroupID.DDL_ANNOTATIONS);
  /** Rules in those groups that change identifiers, comments or code. */
  static final Set<RuleID> NOT_LAYOUT = EnumSet.of(RuleID.CDS_TEST_CLASS_LINES,
      RuleID.CAMEL_CASE_NAME, RuleID.CAMEL_CASE_IN_CDS_TEST, RuleID.DDL_CAMEL_CASE_NAME,
      RuleID.DDL_TYPO, RuleID.DDL_ANNO_NESTING);
  /** Layout rules in other groups. */
  static final Set<RuleID> EXTRA = EnumSet.of(RuleID.LOGICAL_OPERATOR_POSITION);

  /** The default profile reduced to layout rules, with their default options,
   * except that generated "do not include other source files here" comments
   * are kept. */
  static Profile layoutProfile() {
    Profile p = Profile.createDefault();
    for (Rule r : p.getAllRules()) {
      r.isActive = (LAYOUT_GROUPS.contains(r.getGroupID()) && !NOT_LAYOUT.contains(r.getID()))
          || EXTRA.contains(r.getID());
    }
    for (ConfigValue c : p.getRule(RuleID.EMPTY_LINES_IN_CLASS_DEFINITION).getConfigValues()) {
      if (c.settingName.equals("RemoveIncludeWarnings")) ((ConfigBoolValue) c).setValue(false);
    }
    return p;
  }

  /** Cleaned code, or null on a parse, cleanup or check error. */
  static String clean(Profile p, String name, String code) {
    Task t = Job.createForSingleCodeDocument(
        ParseParams.createForWholeCode(name, code, ABAP.NEWEST_RELEASE),
        CleanupParams.createForProfile(p, false)).run();
    if (t.getParseError() != null || t.getCleanupError() != null || t.getCheckError() != null) {
      return null;
    }
    return t.getResultingCode().toString();
  }

  static String norm(String s) {
    return s.replaceAll("\\s+", "").toUpperCase(Locale.ROOT);
  }

  public static void main(String[] a) throws Exception {
    Program.initialize(null, "");
    switch (a[0]) {
      case "profile" -> {
        for (Rule r : layoutProfile().getRulesSortedByGroup()) {
          if (r.isActive) System.out.println(r.getID());
        }
      }
      case "format" -> {
        String code = Files.readString(Path.of(a[1]));
        String out = clean(layoutProfile(), a[1], code);
        if (out == null) throw new IllegalStateException("ABAP Cleaner failed on " + a[1]);
        Files.writeString(Path.of(a[2]), out);
      }
      case "format-all" -> {
        Profile p = layoutProfile();
        List<String> files = Files.readAllLines(Path.of(a[1]));
        for (int i = 0; i < files.size(); i++) {
          String code = Files.readString(Path.of(files.get(i)));
          String out = clean(p, files.get(i), code);
          Files.writeString(Path.of(a[2], i + ".abap"), out == null ? code : out);
        }
      }
      case "scan" -> {
        List<String> files = Files.readAllLines(Path.of(a[1]));
        List<String> codes = new ArrayList<>();
        for (String f : files) codes.add(Files.readString(Path.of(f)));
        Profile p = Profile.createDefault();
        StringBuilder sb = new StringBuilder("# files\t" + files.size() + "\ngroup\tid\tname\tlayout_files\ttext_files\n");
        for (Rule rule : p.getRulesSortedByGroup()) {
          for (Rule r : p.getAllRules()) r.isActive = r == rule;
          int layout = 0, text = 0;
          for (int i = 0; i < codes.size(); i++) {
            String out = clean(p, files.get(i), codes.get(i));
            if (out == null || out.equals(codes.get(i))) continue;
            if (norm(out).equals(norm(codes.get(i)))) layout++; else text++;
          }
          sb.append(rule.getGroupID()).append('\t').append(rule.getID()).append('\t')
              .append(rule.getDisplayName()).append('\t').append(layout).append('\t')
              .append(text).append('\n');
          System.err.println(rule.getID() + " " + layout + " " + text);
        }
        Files.writeString(Path.of(a[2]), sb.toString());
      }
      default -> throw new IllegalArgumentException(a[0]);
    }
  }
}
