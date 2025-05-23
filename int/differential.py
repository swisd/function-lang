from fontTools.misc.cython import returns


def dadd(a, b):
    if not a == b:
        return a + b
    else:
        return 0


def dsub(a, b):
    if not a == b:
        return a - b
    else:
        return 0


def dmul(a, b):
    if not a == b:
        return a * b
    else:
        return 0


def ddiv(a, b):
    if not a == b:
        return a / b
    else:
        return 0


def dpow(a, b):
    if not a == b:
        return a ** b
    else:
        return 0
