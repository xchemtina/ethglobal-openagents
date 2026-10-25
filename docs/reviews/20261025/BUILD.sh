#!/bin/sh
set -eu
cd "$(dirname "$0")"
engine=${LATEX_ENGINE:-pdflatex}
"$engine" -interaction=nonstopmode -halt-on-error REVIEW.tex
"$engine" -interaction=nonstopmode -halt-on-error REVIEW.tex
