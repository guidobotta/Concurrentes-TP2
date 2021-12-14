import sys
#1,1.00,1.00
def random_result(id):
    dos_decimales = "{:.2f}".format(id)
    return f"{id},{dos_decimales},{dos_decimales}"

def main():
    numero = sys.argv[1]
    N = 100 if len(sys.argv) <= 2 else int(sys.argv[2])

    with open(f"./alglobo/files/example-{numero}.csv", 'w+') as archivo:
        for i in range(1,N+1):
            archivo.write(random_result(i))
            archivo.write('\n')

main()