use crate::customer::Customer;
use crate::messages::DeliveryAccepted;
use crate::messages::OrderCanceled;
use crate::messages::OrderConfirmed;
use crate::messages::OrderReadyToPickup;
use crate::messages::PreparingOrder;
use crate::messages::RecoverDelivery;
use crate::messages::RestaurantOrderResponse;
use crate::messages::StatusUpdate;
use crate::messages::{HandshakeReceived, NearbyRestaurants, SaveLoginInfo};
use actix::{Actor, Addr, Context, StreamHandler};
use app_utils::utils::{deserialize_payload, deserialize_tcp_message};
use log::error;
use server::apps_info::restaurant_info::RestaurantData;
use std::io::Error;
use std::net::SocketAddr;

pub struct TcpReceiver {
    /// Dirección de socket asociada
    pub addr: SocketAddr,
    /// Direccion del actor del comensal
    pub client_addr: Addr<Customer>,
}

impl Actor for TcpReceiver {
    type Context = Context<Self>;
}

impl StreamHandler<Result<String, Error>> for TcpReceiver {
    fn handle(&mut self, msg: Result<String, Error>, _ctx: &mut Context<Self>) {
        if let Ok(line) = msg {
            let line = line.trim();

            if line == "HANDSHAKE" {
                // Enviar ACK al servidor
                self.client_addr.do_send(HandshakeReceived);
            } else {
                let (title, payload) = deserialize_tcp_message(&line);
                match title.as_str() {
                    "login_successful" => {
                        let Ok(customer_server_id) = deserialize_payload::<u64>(&payload, "id")
                        else {
                            error!("Error al deserializar el id del cliente");
                            return;
                        };

                        let save_loging_info = SaveLoginInfo {
                            id: customer_server_id,
                        };

                        if let Err(e) = self.client_addr.try_send(save_loging_info) {
                            error!("Error al enviar el mensaje: {}", e);
                        }
                    }
                    "nearby_restaurants" => {
                        let Ok(nearby_restaurants) = deserialize_payload::<Vec<RestaurantData>>(
                            &payload,
                            "nearby_restaurants",
                        ) else {
                            error!("Error al deserializar el campo nearby_restaurants");
                            return;
                        };

                        //info!("Restaurant info: {:?}", nearby_restaurants);

                        let restaurants = NearbyRestaurants { nearby_restaurants };

                        if let Err(e) = self.client_addr.try_send(restaurants) {
                            error!("Error al enviar el mensaje: {}", e);
                        }
                    }
                    "prepare_order_response" => {
                        let Ok(restaurant_response) =
                            deserialize_payload::<bool>(&payload, "accepted")
                        else {
                            error!(
                                "Error al deserializar la respuesta de autorizacion del gateway."
                            );
                            return;
                        };
                        let response = RestaurantOrderResponse {
                            accepted: restaurant_response,
                        };

                        if let Err(e) = self.client_addr.try_send(response) {
                            error!("Error al enviar el mensaje: {}", e);
                        }
                    }
                    "order_confirmed" => {
                        let Ok(order_id) = deserialize_payload::<u64>(&payload, "order_id") else {
                            error!(
                                "Error al deserializar la respuesta de confirmacion del restaurante."
                            );
                            return;
                        };

                        let order_status = OrderConfirmed { order_id };

                        if let Err(e) = self.client_addr.try_send(order_status) {
                            error!("Error al enviar el mensaje: {}", e);
                        }
                    }
                    "order_canceled" => {
                        let order_status = OrderCanceled {};

                        if let Err(e) = self.client_addr.try_send(order_status) {
                            error!("Error al enviar el mensaje: {}", e);
                        }
                    }
                    "preparing_order" => {
                        let Ok(order_id) = deserialize_payload::<u64>(&payload, "order_id") else {
                            error!(
                                "Error al deserializar la actualizacion de status del pedido del restaurante"
                            );
                            return;
                        };

                        let order_status = PreparingOrder { order_id };

                        if let Err(e) = self.client_addr.try_send(order_status) {
                            error!("Error al enviar el mensaje: {}", e);
                        }
                    }
                    "order_ready_for_pickup" => {
                        let Ok(order_id) = deserialize_payload::<u64>(&payload, "order_id") else {
                            error!(
                                "Error al deserializar la actualizacion de status del pedido del restaurante"
                            );
                            return;
                        };

                        let order_status = OrderReadyToPickup { order_id };

                        if let Err(e) = self.client_addr.try_send(order_status) {
                            error!("Error al enviar el mensaje: {}", e);
                        }
                    }
                    "delivery_accepted" => {
                        let order_id = deserialize_payload::<u64>(&payload, "order_id")
                            .expect("delivery_accepted sin order_id");
                        let delivery_position =
                            deserialize_payload::<(u64, u64)>(&payload, "delivery_position")
                                .expect("delivery_accepted sin delivery_position");
                        let delivery_socket =
                            deserialize_payload::<String>(&payload, "delivery_socket")
                                .expect("delivery_accepted sin delivery_socket");
                        let delivery_name =
                            deserialize_payload::<String>(&payload, "delivery_name")
                                .expect("delivery_accepted sin delivery_name");
                        let msg = DeliveryAccepted {
                            order_id,
                            delivery_position,
                            delivery_socket,
                            delivery_name,
                        };
                        let _ = self.client_addr.try_send(msg);
                    }
                    "status_update" => {
                        let message = deserialize_payload::<String>(&payload, "message")
                            .expect("status_update sin message");
                        let _ = self.client_addr.try_send(StatusUpdate { message });
                    }
                    "recovery_delivery" => {
                        let order_id = deserialize_payload::<u64>(&payload, "order_id")
                            .expect("recover_delivery sin order_id");
                        let _ = self.client_addr.try_send(RecoverDelivery { order_id });
                    }

                    _ => {
                        error!("[CUSTOMER]: Mensaje {title} invalido");
                    }
                }
            }
        }
    }
}
