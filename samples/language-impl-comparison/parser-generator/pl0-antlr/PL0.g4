grammar PL0;

program : block '.' EOF ;

block
  : constDecl? varDecl? statement
  ;

constDecl
  : 'const' ident '=' NUMBER (',' ident '=' NUMBER)* ';'
  ;

varDecl
  : 'var' ident (',' ident)* ';'
  ;

statement
  : ident ':=' expression
  | 'call' ident
  | 'begin' statement (';' statement)* 'end'
  | 'if' condition 'then' statement
  | 'while' condition 'do' statement
  | 'write' expression
  | 'skip'
  ;

condition
  : 'odd' expression
  | expression relop expression
  ;

expression
  : addop? term (addop term)*
  ;

term
  : factor (mulop factor)*
  ;

factor
  : ident
  | NUMBER
  | '(' expression ')'
  ;

relop : '=' | '<>' | '<' | '<=' | '>' | '>=' ;
addop : '+' | '-' ;
mulop : '*' | '/' ;

ident : IDENT ;

NUMBER : [0-9]+ ;
IDENT : [a-zA-Z][a-zA-Z0-9_]* ;
WS : [ \t\r\n]+ -> skip ;
