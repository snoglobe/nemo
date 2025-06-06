#!/usr/bin/env python3
"""
Compiler for the language specified in spec.md
Compiles source files to C code.
"""

import sys
import os
import argparse
from pathlib import Path

from lexer import Lexer, TokenType
from parser import Parser
from type_checker import TypeChecker
from codegen import CodeGenerator
from errors import CompilerError


def compile_file(input_path: Path, output_path: Path = None) -> bool:
    """Compile a single source file to C."""
    try:
        # Read source file
        with open(input_path, 'r', encoding='utf-8') as f:
            source = f.read()
        
        # Determine output path
        if output_path is None:
            output_path = input_path.with_suffix('.c')
        
        print(f"Compiling {input_path} -> {output_path}")
        
        # Lexical analysis
        lexer = Lexer(source, str(input_path))
        tokens = lexer.tokenize()
        
        # Parsing
        parser = Parser(tokens, str(input_path))
        ast = parser.parse()
        
        # Type checking
        type_checker = TypeChecker()
        type_checker.check_program(ast)
        
        # Code generation
        code_gen = CodeGenerator()
        c_code = code_gen.generate(ast)
        
        # Write output
        with open(output_path, 'w', encoding='utf-8') as f:
            f.write(c_code)
        
        print(f"Successfully compiled to {output_path}")
        return True
        
    except CompilerError as e:
        print(f"Compilation error: {e}", file=sys.stderr)
        return False
    except Exception as e:
        print(f"Internal compiler error: {e}", file=sys.stderr)
        import traceback
        traceback.print_exc()
        return False


def main():
    parser = argparse.ArgumentParser(description="Compiler for the spec.md language")
    parser.add_argument('input', type=Path, help='Input source file')
    parser.add_argument('-o', '--output', type=Path, help='Output C file (default: input.c)')
    parser.add_argument('--emit-ast', action='store_true', help='Print AST to stdout')
    
    args = parser.parse_args()
    
    if not args.input.exists():
        print(f"Error: Input file '{args.input}' not found", file=sys.stderr)
        sys.exit(1)
    
    success = compile_file(args.input, args.output)
    sys.exit(0 if success else 1)


if __name__ == '__main__':
    main()