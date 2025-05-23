import sys

from lark import Lark, Tree, Token
import re
import math
import dmath
import differential

grammar = r"""
    // operators as tokens
    PLUS:  "+"
    MINUS: "-"
    TIMES: "*"
    DIV:   "/"
    POW:   "^"

    %import common.CNAME -> IDENT
    %import common.NUMBER
    %ignore " "    // skip spaces
    %ignore "\t"
    %ignore "\n"

    start: statement

    ?statement: assignment
          | function_def
          | expr

    
    assignment: IDENT "=" expr
    
    function_def: func_def_head "=" expr   -> func_def
    func_def_head: IDENT "(" IDENT ")"
    print_stmt: "print" "(" expr ")"

    ?expr: sum
    ?sum: product ( (PLUS|MINUS) product )*
    ?product: power ( (TIMES|DIV) power )*
    ?power: unary ( POW power )?
    ?unary: (PLUS|MINUS)? primary
    ?primary: NUMBER      -> number
        | IDENT       -> var
        | IDENT "(" expr_list? ")" -> func_call
        | "(" expr ")"


    expr_list: expr ("," expr)*
    
"""

CONSTANTS = {
    "pi": math.pi,
    "e": math.e,
    "inf": math.inf,
    "nan": math.nan,
    "tau": math.tau,
}

BUILTINS = {
    "sin": dmath.dsin,
    "cos": dmath.dcos,
    "tan": dmath.dtan,
    "log": math.log,  # log(x [, base])
    "max": max,
    "min": min,
    "trunc": math.trunc,
    "round": round,
    "cbrt": math.cbrt,
    "sqrt": math.sqrt,
    "ceil": math.ceil,
    "floor": math.floor,
    "fact": math.factorial,
    "log2": math.log2,
    "log10": math.log10,
    "sinh": dmath.dsinh,
    "cosh": dmath.dcosh,
    "tanh": dmath.dtanh,
    "sin1": dmath.dasin,
    "cos1": dmath.dacos,
    "tan1": dmath.datan,
    "dadd": differential.dadd,
    "dsub": differential.dsub,
    "dmul": differential.dmul,
    "ddiv": differential.ddiv,
    "dexp": differential.dpow,
}

parser = Lark(grammar, parser="lalr", propagate_positions=True)

vars_table = {}
funcs_table = {}


def eval_expr(tree: Tree, local_vars):
    """Recursively evaluate an expr-Tree with given locals."""
    t = tree.data
    children = list(tree.children)

    if t == "number":
        return float(children[0])
    if t == "var":
        name = str(children[0])
        if name == "pi":
            return math.pi
        if name == "e":
            return math.e
        if name in local_vars:
            return local_vars[name]
        elif name in vars_table:
            return vars_table[name]
        elif name in CONSTANTS:
            return CONSTANTS[name]
        raise NameError(f"Undefined variable: {name}")

    if t == "sum":
        acc = eval_expr(children[0], local_vars)
        for op_tok, subtree in zip(children[1::2], children[2::2]):
            rhs = eval_expr(subtree, local_vars)
            if op_tok.type == "PLUS":
                acc += rhs
            else:  # MINUS
                acc -= rhs
        return acc

    if t == "product":
        acc = eval_expr(children[0], local_vars)
        for op_tok, subtree in zip(children[1::2], children[2::2]):
            rhs = eval_expr(subtree, local_vars)
            if op_tok.type == "TIMES":
                acc *= rhs
            else:  # DIV
                acc /= rhs
        return acc

    if t == "power":
        # Evaluate the base first
        result = eval_expr(children[0], local_vars)

        # Then walk the (token, subtree) pairs in items[1::2] and items[2::2]
        for op_tok, subtree in zip(children[1::2], children[2::2]):
            rhs = eval_expr(subtree, local_vars)
            if op_tok.type == "POW":
                result = result ** rhs
            else:
                raise RuntimeError(f"Unexpected operator in power(): {op_tok}")
        return result

    if t == "unary":
        if len(children) == 1:
            return eval_expr(children[0], local_vars)
        sign = children[0].type
        val = eval_expr(children[1], local_vars)
        return val if sign == "PLUS" else -val

    if t == "func_call":
        name = str(children[0])
        # Flatten expr_list, if that’s what children[1] is:
        raw_args = []
        if len(children) == 2 and isinstance(children[1], Tree) and children[1].data == "expr_list":
            raw_args = children[1].children
        else:
            raw_args = children[1:]

        args = [eval_expr(c, local_vars) for c in raw_args]
        # user-defined?
        if name in BUILTINS:
            return BUILTINS[name](*args)
        elif name in funcs_table:
            param, body = funcs_table[name]
            # shadow param
            new_locals = dict(local_vars)
            new_locals[param] = args[0] if args else None
            return eval_expr(body, new_locals)
        else:
            raise NameError(f"Unknown function: {name}")

    # parenthesized just forwards
    if t == "expr":
        return eval_expr(children[0], local_vars)

    if t == "product_implicit":
        acc = eval_expr(children[0], local_vars)
        for subtree in children[1:]:
            rhs = eval_expr(subtree, local_vars)
            acc *= rhs
        return acc

        raise RuntimeError(f"Unhandled tree node: {t}")

def run_file(filename):
    with open(filename) as f:
        for line in f:
            repl(line)

def repl(line):
    # 1) Implicit multiplication hack
    line = re.sub(r'(\d)([A-Za-z\(])', r'\1*\2', line)
    line = re.sub(r'(\))(\d|[A-Za-z\(])', r'\1*\2', line)
    if not line:
        pass
    if line in {"exit", "quit"}:
        sys.exit(0)

    tree = parser.parse(line)
    stmt = tree.children[0]

    if stmt.data == "assignment":
        name = str(stmt.children[0])
        val = eval_expr(stmt.children[1], {})
        vars_table[name] = val
        print("=", val)

    elif stmt.data == "function_def":
        name = str(stmt.children[0])
        param = str(stmt.children[1])
        body = stmt.children[2]
        funcs_table[name] = (param, body)
        # no immediate evaluation


    elif stmt.data == "func_def":

        func_head = stmt.children[0]  # func_def_head
        name = str(func_head.children[0])
        param = str(func_head.children[1])
        body = stmt.children[1]
        funcs_table[name] = (param, body)

    elif stmt.data == "print_stmt":
        val = eval_expr(stmt.children[0], {})
        print(val)

    else:  # bare expression
        val = eval_expr(stmt, {})
        print("=", val)



def main():
    try:
        _ = sys.argv[1]
        try:
            run_file(sys.argv[1])
        except Exception as e:
            print("Error:", e)
        _ = input()
        sys.exit(0)
    except:
        while True:
            try:
                line = input("> ").strip()
                repl(line)
            except Exception as e:
                print("Error:", e)

if __name__ == "__main__":
    main()
