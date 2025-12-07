package com.sylketech.rokbattles.web;

import jakarta.servlet.http.HttpServletResponse;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;
import org.springframework.security.config.annotation.web.builders.HttpSecurity;
import org.springframework.security.config.annotation.web.configuration.EnableWebSecurity;
import org.springframework.security.config.annotation.web.configurers.AbstractHttpConfigurer;
import org.springframework.security.oauth2.client.registration.ClientRegistrationRepository;
import org.springframework.security.oauth2.client.web.DefaultOAuth2AuthorizationRequestResolver;
import org.springframework.security.oauth2.client.web.OAuth2AuthorizationRequestCustomizers;
import org.springframework.security.web.SecurityFilterChain;

@Configuration
@EnableWebSecurity
public class SecurityConfiguration {
    @Value("${app.frontend-url:http://localhost:8080}")
    private String frontendUrl;

    @Bean
    public SecurityFilterChain securityFilterChain(HttpSecurity http, ClientRegistrationRepository clientRegistrationRepository) throws Exception {
        DefaultOAuth2AuthorizationRequestResolver resolver = new DefaultOAuth2AuthorizationRequestResolver(
                clientRegistrationRepository,
                "/api/oauth2/authorization"
        );
        resolver.setAuthorizationRequestCustomizer(OAuth2AuthorizationRequestCustomizers.withPkce());

        http
                .authorizeHttpRequests(auth -> auth
                        .requestMatchers("/api/oauth2/**").permitAll()
                        .requestMatchers("/api/**").authenticated()
                        .anyRequest().permitAll()
                )
                .oauth2Login(oauth -> oauth
                        .authorizationEndpoint(config -> config
                                .baseUri("/api/oauth2/authorization")
                                .authorizationRequestResolver(resolver)
                        )
                        .redirectionEndpoint(config -> config.baseUri("/api/oauth2/callback/*"))
                        .successHandler((_, response, _) -> response.sendRedirect(this.frontendUrl))
                )
                .logout(logout -> logout
                        .logoutUrl("/api/oauth2/logout")
                        .logoutSuccessHandler((_, response, _) -> response.sendRedirect(this.frontendUrl))
                )
                .exceptionHandling(exceptions -> exceptions
                        .authenticationEntryPoint((_, response, _) -> response.sendError(HttpServletResponse.SC_UNAUTHORIZED))
                )
                .csrf(AbstractHttpConfigurer::disable)
                .httpBasic(AbstractHttpConfigurer::disable);

        return http.build();
    }
}
