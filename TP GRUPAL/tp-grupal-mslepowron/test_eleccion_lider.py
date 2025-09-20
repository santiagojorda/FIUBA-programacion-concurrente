import subprocess
import time
import os
import signal
from socket import *

# Para printear con colores
OK = '\033[92m'
FAIL = '\033[91m'
ENDCOLOR = '\033[0m'

TCP_PORT_RANGE = 10000
UDP_PORT_RANGE = 5000
MAX_SERVERS = 5

#############################################  TESTS  ############################################################
def test_eleccion_lider():
    udp_socket = socket(AF_INET, SOCK_DGRAM)
    stop_all_servers()
    
    print("Esperando...")
    time.sleep(1)
    print("Levantando servers...")
    start_leader_with_id(1)
    time.sleep(1)
    print("Levantando replicas")
    start_replica_with_id(2)
    start_replica_with_id(3)
    start_replica_with_id(4)
    start_replica_with_id(5)
    time.sleep(1)

    print("Apago el server lider")
    stop_server_with_id(1)

    print("Esperando que se resuelva el algoritmo de eleccion")
    time.sleep(5)
    print("Mando requests preguntando el lider a cada server")
    send_get_leader_id_requests(udp_socket)
    print("Recibo las respuestas de las requests")
    responses = recv_get_leader_id_responses(udp_socket, 4)
    expected_leader_id = 5
    check_responses(responses, expected_leader_id)
    return responses

def test_eleccion_lider_caida_replica():
    udp_socket = socket(AF_INET, SOCK_DGRAM)
    stop_all_servers()
    
    print("Esperando...")
    time.sleep(1)
    print("Levantando servers...")
    start_leader_with_id(1)
    time.sleep(1)
    print("Levantando replicas")
    start_replica_with_id(2)
    start_replica_with_id(3)
    start_replica_with_id(4)
    start_replica_with_id(5)
    time.sleep(1)
    
    print("Apago el server replica de id 3")
    stop_server_with_id(3)

    time.sleep(1)

    print("Apago el server lider")
    stop_server_with_id(1)

    print("Esperando que se resuelva el algoritmo de eleccion")
    time.sleep(5)

    print("Le pido el lider a las replicas")
    send_get_leader_id_requests(udp_socket)

    responses = recv_get_leader_id_responses(udp_socket, 3)
    expected_leader_id = 5
    stop_all_servers()
    check_responses(responses, expected_leader_id)
    return responses

def test_eleccion_lider_caida_varias_replicas():
    udp_socket = socket(AF_INET, SOCK_DGRAM)
    stop_all_servers()
    
    print("Esperando...")
    time.sleep(1)
    print("Levantando servers...")
    start_leader_with_id(1)
    time.sleep(1)
    print("Levantando replicas")
    start_replica_with_id(2)
    start_replica_with_id(3)
    start_replica_with_id(4)
    start_replica_with_id(5)
    time.sleep(1)
    
    print("Apago el servers replicas de ids 3 y 5")
    stop_server_with_id(3)
    stop_server_with_id(5)

    time.sleep(1)

    print("Apago el server lider")
    stop_server_with_id(1)

    print("Esperando que se resuelva el algoritmo de eleccion")
    time.sleep(5)

    print("Le pido el lider a las replicas")
    send_get_leader_id_requests(udp_socket)

    responses = recv_get_leader_id_responses(udp_socket, 2)
    expected_leader_id = 4
    stop_all_servers()
    check_responses(responses, expected_leader_id)
    return responses   

def test_eleccion_lider_queda_replica_sola():
    udp_socket = socket(AF_INET, SOCK_DGRAM)
    stop_all_servers()
    
    print("Esperando...")
    time.sleep(1)
    print("Levantando servers...")
    start_leader_with_id(1)
    time.sleep(1)
    print("Levantando replicas")
    start_replica_with_id(2)
    start_replica_with_id(3)
    start_replica_with_id(4)
    start_replica_with_id(5)
    time.sleep(1)
    
    print("Apago el servers replicas de ids 3 y 5")
    stop_server_with_id(3)
    stop_server_with_id(4)
    stop_server_with_id(5)

    time.sleep(1)

    print("Apago el server lider")
    stop_server_with_id(1)

    print("Esperando que se resuelva el algoritmo de eleccion")
    time.sleep(10)

    print("Le pido el lider a las replicas")
    send_get_leader_id_requests(udp_socket)

    responses = recv_get_leader_id_responses(udp_socket, 1)
    expected_leader_id = 2
    stop_all_servers()
    check_responses(responses, expected_leader_id)
    return responses   






#############################################  UTILS  ############################################################
def recv_udp_message(udp_socket):
    leader_id, address = udp_socket.recvfrom(1)
    print(f"Recibo leader id {int(leader_id)} de parte de {address}")
    return int(leader_id), address

def send_udp_message(udp_socket, message, addr):
    print(f"Mandando mensaje = {message} a {addr}")
    udp_socket.sendto(str.encode(message), addr)

def kill_process_running_on_port(port):
    print(f"Matando al proceso escuchando en el puerto {port}")
    subprocess.run(f"lsof -ti:{port} | xargs kill -9".split(" "),shell=True, capture_output=True)

def stop_server_with_id(id):
    kill_process_on_port(TCP_PORT_RANGE + id)
    kill_process_on_port(UDP_PORT_RANGE + id)

def kill_process_on_port(port):
    result = subprocess.run(
        ["lsof", "-t", f"-i:{port}"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True
    )
    
    pid = result.stdout.strip().split("\n")[0]
    
    if pid:
        print(f"Killing process with PID {pid} on port {port}")
        os.kill(int(pid), signal.SIGTERM)

def stop_all_servers():
    for i in range(1, 6):
        try:
            kill_process_on_port(TCP_PORT_RANGE + i)
            kill_process_on_port(UDP_PORT_RANGE + i)
        except Exception as e:
            print(e)

def start_leader_with_id(id):
    subprocess.Popen([f"cd server && cargo run {id} true"], shell=True, stdout=subprocess.PIPE)

def start_replica_with_id(id):
    subprocess.Popen([f"cd server && cargo run {id} false"], shell=True, stdout=subprocess.PIPE)

def send_get_leader_id_requests(udp_socket):
    for i in range(1, MAX_SERVERS + 1):
        send_udp_message(udp_socket, "{\"name\": \"get_leader\", \"payload\": {}}", ("127.0.0.1",5000+i))

def recv_get_leader_id_responses(udp_socket, number_of_responses):
    responses = []
    for i in range(0, number_of_responses):
        id, address = recv_udp_message(udp_socket)
        responses.append((id, address))
    return responses

def check_responses(responses, expected_leader_id):
    print(f"\n\n\n************************ RESULTADO ***********************")
    print(f"Lider esperado: {expected_leader_id}")
    for response in responses:
        id_leader = response[0]
        address = response[1]
        print(f"El servidor {address} respondió que su lider es {id_leader}")
        try:
            assert(id_leader == expected_leader_id)
        except Exception as e:
            print(f"{FAIL}Test falló. ID de lider esperado: {expected_leader_id}\nID de lider obtenido: {id_leader}{ENDCOLOR}")
            return
    print(f"{OK}Test exitoso{ENDCOLOR}")

def main():
    
    responses_test_1 = test_eleccion_lider()
    
    stop_all_servers()
    
    responses_test_2 = test_eleccion_lider_caida_replica()
    
    time.sleep(1)
    
    responses_test_3 = test_eleccion_lider_caida_varias_replicas()
    
    time.sleep(1)

    responses_test_4 = test_eleccion_lider_queda_replica_sola()
    expected_leader_test_1 = 5
    expected_leader_test_2 = 5
    expected_leader_test_3 = 4
    expected_leader_test_4 = 2
    print(f"\n\n\n************************ RESULTADOS TESTS ************************")
    check_responses(responses_test_1, expected_leader_test_1)
    check_responses(responses_test_2, expected_leader_test_2)
    check_responses(responses_test_3, expected_leader_test_3)
    check_responses(responses_test_4, expected_leader_test_4)



main()