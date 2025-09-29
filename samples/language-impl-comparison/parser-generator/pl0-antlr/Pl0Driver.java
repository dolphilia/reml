import org.antlr.v4.runtime.*;
import org.antlr.v4.runtime.tree.ParseTree;

public final class Pl0Driver {
  public static void main(String[] args) throws Exception {
    String source = String.join(" ", args);
    CharStream input = CharStreams.fromString(source);
    PL0Lexer lexer = new PL0Lexer(input);
    CommonTokenStream tokens = new CommonTokenStream(lexer);
    PL0Parser parser = new PL0Parser(tokens);
    ParseTree tree = parser.program();
    System.out.println(tree.toStringTree(parser));
  }
}
