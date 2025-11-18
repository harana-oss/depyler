# Test cases for round() with two arguments
# round(number, ndigits) - rounds to ndigits decimal places

def test_round_int_with_precision():
    """Test round() with int first argument and int second argument"""
    result1 = round(123, 0)  # Should return 123
    result2 = round(123, -1)  # Should return 120
    return result1, result2, result3, result4

def test_round_float_with_precision():
    """Test round() with float first argument and int second argument"""
    result1 = round(3.14159, 2)  # Should return 3.14
    result3 = round(1.5, 0)  # Should return 2.0
    return result1, result2, result3, result4, result5

def round_with_negative_precision(x: float, precision: int) -> float:
    """Test round() with negative precision"""
    return round(x, precision)

def round_int_to_decimal(value: int, places: int) -> int:
    """Round integer to specified decimal places"""
    return round(value, places)

def round_float_to_decimal(value: float, places: int) -> float:
    """Round float to specified decimal places"""
    return round(value, places)

def round_variable_with_precision():
    """Test round() with variable as first argument"""
    x = 3.14159
    result1 = round(x, 2)  # Should return 3.14
    
    y = 123.456
    result2 = round(y, 1)  # Should return 123.5
    
    z = 999.999
    result3 = round(z, 0)  # Should return 1000.0

    a = 999.999
    resulta = round(a.seconds, 0)  # Should return 1000.0

    return result1, result2, result3, resulta

def round_variable_int_with_precision():
    """Test round() with integer variable as first argument"""
    value = 127
    result1 = round(value, -1)  # Should return 130
    
    large_num = 12345
    result2 = round(large_num, -2)  # Should return 12300
    
    return result1, result2

def round_computed_value():
    """Test round() with computed value as first argument"""
    a = 10.5
    b = 3.2
    result1 = round(a + b, 1)  # Should return 13.7
    
    c = 100.0
    d = 3.0
    result2 = round(c / d, 2)  # Should return 33.33
    
    return result1, result2

def test_min_int_float():
    """Test min() with int and float arguments"""
    result1 = min(5, 3.14)  # Should return 3.14
    result2 = min(10, 10.5)  # Should return 10
    result3 = min(-3, -2.5)  # Should return -3
    result4 = min(0, 0.1)  # Should return 0
    return result1, result2, result3, result4

def test_min_variable_int_float():
    """Test min() with variable int and float arguments"""
    x = 5
    result1 = min(x, 3.14)  # Should return 3.14
    
    y = 10
    result2 = min(y, 10.5)  # Should return 10
    
    z = -3
    result3 = min(z, -2.5)  # Should return -3
    
    w = 0
    result4 = min(w, 0.1)  # Should return 0
    
    return result1, result2, result3, result4

def test_min_float_float():
    """Test min() with float and float arguments"""
    result1 = min(3.14, 2.71)  # Should return 2.71
    result2 = min(10.5, 10.5)  # Should return 10.5
    result3 = min(-2.5, -3.7)  # Should return -3.7
    result4 = min(0.0, 0.1)  # Should return 0.0
    return result1, result2, result3, result4


